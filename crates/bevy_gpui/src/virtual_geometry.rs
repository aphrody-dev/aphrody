//! Nanite-inspired Virtual Geometry: partitions triangle meshes into bounded 64-vertex / 124-triangle
//! [`MeshletCluster`] blocks with bounding spheres, backface normal cones, and screen-space LOD culling.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::ecs_world::{CameraRig, Vec3f};

pub const MAX_MESHLET_VERTICES: usize = 64;
pub const MAX_MESHLET_TRIANGLES: usize = 124;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable, Serialize, Deserialize)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MeshAsset {
    pub name: String,
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshletCluster {
    pub cluster_id: u32,
    pub vertex_offset: u32,
    pub vertex_count: u32,
    pub triangle_offset: u32,
    pub triangle_count: u32,
    pub bounding_center: [f32; 3],
    pub bounding_radius: f32,
    pub cone_axis: [f32; 3],
    pub cone_cutoff: f32,
    pub lod_error_px: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MeshletCullingStats {
    pub total_clusters: usize,
    pub visible_clusters: usize,
    pub frustum_culled: usize,
    pub cone_backface_culled: usize,
    pub visible_triangles: usize,
}

/// Generates a UV sphere mesh suitable for PBR / MaterialX preview and Nanite meshlet clustering.
#[must_use]
pub fn build_uv_sphere(radius: f32, rings: u32, sectors: u32) -> MeshAsset {
    let r_count = rings.max(4);
    let s_count = sectors.max(6);
    let mut vertices = Vec::with_capacity(((r_count + 1) * (s_count + 1)) as usize);
    let mut indices = Vec::with_capacity((r_count * s_count * 6) as usize);

    for r in 0..=r_count {
        let v = r as f32 / r_count as f32;
        let phi = v * std::f32::consts::PI;
        let sin_phi = phi.sin();
        let cos_phi = phi.cos();

        for s in 0..=s_count {
            let u = s as f32 / s_count as f32;
            let theta = u * std::f32::consts::TAU;
            let nx = sin_phi * theta.cos();
            let ny = cos_phi;
            let nz = sin_phi * theta.sin();
            vertices.push(MeshVertex {
                position: [nx * radius, ny * radius, nz * radius],
                normal: [nx, ny, nz],
                uv: [u, v],
            });
        }
    }

    let stride = s_count + 1;
    for r in 0..r_count {
        for s in 0..s_count {
            let i0 = r * stride + s;
            let i1 = i0 + stride;
            indices.extend_from_slice(&[i0, i1, i0 + 1, i0 + 1, i1, i1 + 1]);
        }
    }

    MeshAsset {
        name: format!("uv_sphere_r{r_count}_s{s_count}"),
        vertices,
        indices,
    }
}

/// Partitions a [`MeshAsset`] into GPU-friendly 64-vertex / 124-triangle [`MeshletCluster`] blocks.
#[must_use]
pub fn build_meshlets(mesh: &MeshAsset) -> Vec<MeshletCluster> {
    let mut clusters = Vec::new();
    let total_tris = mesh.indices.len() / 3;
    let mut tri_idx = 0usize;
    let mut cluster_id = 0u32;

    while tri_idx < total_tris {
        let mut local_verts: Vec<u32> = Vec::with_capacity(MAX_MESHLET_VERTICES);
        let start_tri = tri_idx;
        let mut tri_normals: Vec<Vec3f> = Vec::with_capacity(MAX_MESHLET_TRIANGLES);

        while tri_idx < total_tris && (tri_idx - start_tri) < MAX_MESHLET_TRIANGLES {
            let base = tri_idx * 3;
            let (Some(&a), Some(&b), Some(&c)) = (
                mesh.indices.get(base),
                mesh.indices.get(base + 1),
                mesh.indices.get(base + 2),
            ) else {
                break;
            };
            let needed = [a, b, c]
                .iter()
                .filter(|v| !local_verts.contains(v))
                .count();
            if local_verts.len() + needed > MAX_MESHLET_VERTICES && !local_verts.is_empty() {
                break;
            }
            for v in [a, b, c] {
                if !local_verts.contains(&v) {
                    local_verts.push(v);
                }
            }
            if let (Some(va), Some(vb), Some(vc)) = (
                mesh.vertices.get(a as usize),
                mesh.vertices.get(b as usize),
                mesh.vertices.get(c as usize),
            ) {
                let pa = Vec3f::new(va.position[0], va.position[1], va.position[2]);
                let pb = Vec3f::new(vb.position[0], vb.position[1], vb.position[2]);
                let pc = Vec3f::new(vc.position[0], vc.position[1], vc.position[2]);
                tri_normals.push(pb.sub(pa).cross(pc.sub(pa)).normalized());
            }
            tri_idx += 1;
        }

        let mut center = Vec3f::default();
        let mut valid_count = 0usize;
        for &vi in &local_verts {
            if let Some(v) = mesh.vertices.get(vi as usize) {
                center = center.add(Vec3f::new(v.position[0], v.position[1], v.position[2]));
                valid_count += 1;
            }
        }
        if valid_count > 0 {
            center = center.scale(1.0 / valid_count as f32);
        }
        let mut radius = 0.0_f32;
        for &vi in &local_verts {
            if let Some(v) = mesh.vertices.get(vi as usize) {
                let d = Vec3f::new(v.position[0], v.position[1], v.position[2])
                    .sub(center)
                    .length();
                radius = radius.max(d);
            }
        }

        let mut avg_n = Vec3f::default();
        for n in &tri_normals {
            avg_n = avg_n.add(*n);
        }
        let axis = avg_n.normalized();
        let mut min_dot = 1.0_f32;
        for n in &tri_normals {
            min_dot = min_dot.min(axis.dot(*n));
        }
        // Cone cutoff formula for cluster backface rejection: sin(half_angle) approximation.
        let cone_cutoff = if min_dot <= 0.0 {
            1.0
        } else {
            (1.0 - min_dot * min_dot).max(0.0).sqrt()
        };

        let tri_count = (tri_idx - start_tri) as u32;
        clusters.push(MeshletCluster {
            cluster_id,
            vertex_offset: local_verts.first().copied().unwrap_or(0),
            vertex_count: local_verts.len() as u32,
            triangle_offset: start_tri as u32,
            triangle_count: tri_count,
            bounding_center: [center.x, center.y, center.z],
            bounding_radius: radius.max(1e-3),
            cone_axis: [axis.x, axis.y, axis.z],
            cone_cutoff,
            lod_error_px: (radius * 0.15).max(0.05),
        });
        cluster_id = cluster_id.saturating_add(1);
    }

    clusters
}

/// Performs view-frustum + normal-cone backface culling over Nanite-style [`MeshletCluster`] blocks.
#[must_use]
pub fn cull_meshlets(
    clusters: &[MeshletCluster],
    actor_translation: Vec3f,
    camera: &CameraRig,
) -> (Vec<u32>, MeshletCullingStats) {
    let forward = camera.target.sub(camera.eye).normalized();
    let half_fov_cos = (camera.vertical_fov_deg.to_radians() * 0.72).cos();
    let mut visible_ids = Vec::with_capacity(clusters.len());
    let mut stats = MeshletCullingStats {
        total_clusters: clusters.len(),
        ..MeshletCullingStats::default()
    };

    for cluster in clusters {
        let world_center = actor_translation.add(Vec3f::new(
            cluster.bounding_center[0],
            cluster.bounding_center[1],
            cluster.bounding_center[2],
        ));
        let to_cluster = world_center.sub(camera.eye);
        let dist = to_cluster.length();
        if dist - cluster.bounding_radius > camera.far || dist + cluster.bounding_radius < camera.near
        {
            stats.frustum_culled += 1;
            continue;
        }
        if dist > cluster.bounding_radius * 1.5 {
            let dir = to_cluster.scale(1.0 / dist.max(1e-4));
            let angular_slack = (cluster.bounding_radius / dist).clamp(0.0, 1.0);
            if dir.dot(forward) < (half_fov_cos - angular_slack) {
                stats.frustum_culled += 1;
                continue;
            }
        }

        let axis = Vec3f::new(
            cluster.cone_axis[0],
            cluster.cone_axis[1],
            cluster.cone_axis[2],
        );
        let view_to_center = world_center.sub(camera.eye).normalized();
        if cluster.cone_cutoff < 0.99 && view_to_center.dot(axis) > cluster.cone_cutoff {
            stats.cone_backface_culled += 1;
            continue;
        }

        visible_ids.push(cluster.cluster_id);
        stats.visible_clusters += 1;
        stats.visible_triangles += cluster.triangle_count as usize;
    }

    (visible_ids, stats)
}
