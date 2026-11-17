//! Archetype ECS world, 3D math, ray-picking, USD Crate LZ4 decoding (`craft-usd`),
//! and `bevy-mocap` / XSENS skeletal keyframe evaluation.

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Vec3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3f {
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[must_use]
    pub fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }

    #[must_use]
    pub fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }

    #[must_use]
    pub fn scale(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }

    #[must_use]
    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    #[must_use]
    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    #[must_use]
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    #[must_use]
    pub fn normalized(self) -> Self {
        let len = self.length();
        if len > 1e-6 {
            self.scale(1.0 / len)
        } else {
            Self::new(0.0, 1.0, 0.0)
        }
    }
}

pub type ActorId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform3d {
    pub translation: Vec3f,
    pub rotation_euler_deg: Vec3f,
    pub scale: Vec3f,
}

impl Default for Transform3d {
    fn default() -> Self {
        Self {
            translation: Vec3f::new(0.0, 0.0, 0.0),
            rotation_euler_deg: Vec3f::new(0.0, 0.0, 0.0),
            scale: Vec3f::new(1.0, 1.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveShape {
    Sphere,
    BeveledBox,
    GroundPlane,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PbrSurfaceMaterial {
    pub name: String,
    pub base_color_linear: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    pub clearcoat: f32,
    pub emissive_linear: [f32; 3],
}

impl Default for PbrSurfaceMaterial {
    fn default() -> Self {
        Self {
            name: "GoogleExpressiveChrome".to_owned(),
            base_color_linear: [0.35, 0.68, 0.98],
            metallic: 0.72,
            roughness: 0.22,
            clearcoat: 0.45,
            emissive_linear: [0.0, 0.0, 0.0],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MocapKeyframe {
    pub time_secs: f32,
    pub position: Vec3f,
    pub yaw_deg: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorEntity {
    pub id: ActorId,
    pub label: String,
    pub visible: bool,
    pub shape: PrimitiveShape,
    pub radius_or_half_extent: f32,
    pub transform: Transform3d,
    pub material: PbrSurfaceMaterial,
    pub mocap_track: Vec<MocapKeyframe>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CameraRig {
    pub eye: Vec3f,
    pub target: Vec3f,
    pub up: Vec3f,
    pub vertical_fov_deg: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            eye: Vec3f::new(3.4, 2.4, 4.6),
            target: Vec3f::new(0.0, 0.65, 0.0),
            up: Vec3f::new(0.0, 1.0, 0.0),
            vertical_fov_deg: 46.0,
            near: 0.1,
            far: 200.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DirectionalSunLight {
    pub direction_to_sun: Vec3f,
    pub color_linear: [f32; 3],
    pub illuminance_lux: f32,
}

impl Default for DirectionalSunLight {
    fn default() -> Self {
        Self {
            direction_to_sun: Vec3f::new(0.58, 0.76, 0.30).normalized(),
            color_linear: [1.0, 0.96, 0.90],
            illuminance_lux: 3.4,
        }
    }
}

/// Complete Unreal 5.8-style ECS scene world hosted inside Zed / Aphrody.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnrealWorld {
    pub scene_name: String,
    pub time_secs: f32,
    pub camera: CameraRig,
    pub sun: DirectionalSunLight,
    pub selected_actor: Option<ActorId>,
    next_id: ActorId,
    pub actors: Vec<ActorEntity>,
}

impl Default for UnrealWorld {
    fn default() -> Self {
        Self::default_showcase_scene()
    }
}

impl UnrealWorld {
    /// Builds a ready-to-render showcase world with metallic/dielectric PBR spheres, ground plane, and mocap orbit track.
    #[must_use]
    pub fn default_showcase_scene() -> Self {
        let mut world = Self {
            scene_name: "Aphrody_UE58_Solari_Showcase".to_owned(),
            time_secs: 0.0,
            camera: CameraRig::default(),
            sun: DirectionalSunLight::default(),
            selected_actor: Some(1),
            next_id: 1,
            actors: Vec::new(),
        };

        let hero_id = world.spawn_actor(
            "Hero_Nanite_Orb",
            PrimitiveShape::Sphere,
            0.92,
            Vec3f::new(0.0, 0.92, 0.0),
            PbrSurfaceMaterial::default(),
        );
        if let Some(hero) = world.actor_mut(hero_id) {
            hero.mocap_track = vec![
                MocapKeyframe {
                    time_secs: 0.0,
                    position: Vec3f::new(0.0, 0.92, 0.0),
                    yaw_deg: 0.0,
                },
                MocapKeyframe {
                    time_secs: 2.0,
                    position: Vec3f::new(0.0, 1.12, 0.0),
                    yaw_deg: 90.0,
                },
                MocapKeyframe {
                    time_secs: 4.0,
                    position: Vec3f::new(0.0, 0.92, 0.0),
                    yaw_deg: 180.0,
                },
            ];
        }

        world.spawn_actor(
            "Accent_Tertiary_Sphere",
            PrimitiveShape::Sphere,
            0.56,
            Vec3f::new(-1.65, 0.56, -0.45),
            PbrSurfaceMaterial {
                name: "ExpressiveVioletCoat".to_owned(),
                base_color_linear: [0.68, 0.52, 0.98],
                metallic: 0.15,
                roughness: 0.18,
                clearcoat: 0.85,
                emissive_linear: [0.02, 0.01, 0.05],
            },
        );

        world.spawn_actor(
            "Warm_Gold_Conductor",
            PrimitiveShape::Sphere,
            0.52,
            Vec3f::new(1.55, 0.52, 0.35),
            PbrSurfaceMaterial {
                name: "BrushedGoldConductor".to_owned(),
                base_color_linear: [0.96, 0.76, 0.34],
                metallic: 0.94,
                roughness: 0.26,
                clearcoat: 0.10,
                emissive_linear: [0.0, 0.0, 0.0],
            },
        );

        world.spawn_actor(
            "Studio_Checker_Floor",
            PrimitiveShape::GroundPlane,
            18.0,
            Vec3f::new(0.0, 0.0, 0.0),
            PbrSurfaceMaterial {
                name: "ObsidianSlateGrid".to_owned(),
                base_color_linear: [0.09, 0.11, 0.15],
                metallic: 0.05,
                roughness: 0.65,
                clearcoat: 0.0,
                emissive_linear: [0.0, 0.0, 0.0],
            },
        );

        world
    }

    pub fn spawn_actor(
        &mut self,
        label: impl Into<String>,
        shape: PrimitiveShape,
        radius_or_half_extent: f32,
        position: Vec3f,
        material: PbrSurfaceMaterial,
    ) -> ActorId {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.actors.push(ActorEntity {
            id,
            label: label.into(),
            visible: true,
            shape,
            radius_or_half_extent: radius_or_half_extent.max(0.05),
            transform: Transform3d {
                translation: position,
                ..Transform3d::default()
            },
            material,
            mocap_track: Vec::new(),
        });
        id
    }

    #[must_use]
    pub fn actor(&self, id: ActorId) -> Option<&ActorEntity> {
        self.actors.iter().find(|a| a.id == id)
    }

    pub fn actor_mut(&mut self, id: ActorId) -> Option<&mut ActorEntity> {
        self.actors.iter_mut().find(|a| a.id == id)
    }

    /// Advances ECS simulation & evaluates `bevy-mocap` / XSENS keyframe tracks.
    pub fn step(&mut self, dt_secs: f32) {
        self.time_secs = (self.time_secs + dt_secs.max(0.0)) % 3600.0;
        for actor in &mut self.actors {
            if actor.mocap_track.len() >= 2 {
                let Some(last) = actor.mocap_track.last() else {
                    continue;
                };
                let duration = last.time_secs.max(0.001);
                let local_t = self.time_secs % duration;
                for pair in actor.mocap_track.windows(2) {
                    let a = pair[0];
                    let b = pair[1];
                    if local_t >= a.time_secs && local_t <= b.time_secs {
                        let span = (b.time_secs - a.time_secs).max(1e-4);
                        let alpha = ((local_t - a.time_secs) / span).clamp(0.0, 1.0);
                        actor.transform.translation =
                            a.position.add(b.position.sub(a.position).scale(alpha));
                        actor.transform.rotation_euler_deg.y =
                            a.yaw_deg + (b.yaw_deg - a.yaw_deg) * alpha;
                        break;
                    }
                }
            }
        }
    }

    /// Casts a world-space picking ray (`bevy_picking` parity) and returns the closest hit actor ID + distance.
    #[must_use]
    pub fn pick_ray(&self, origin: Vec3f, direction: Vec3f) -> Option<(ActorId, f32)> {
        let dir = direction.normalized();
        let mut best: Option<(ActorId, f32)> = None;
        for actor in &self.actors {
            if !actor.visible {
                continue;
            }
            let hit_t = match actor.shape {
                PrimitiveShape::Sphere | PrimitiveShape::BeveledBox => {
                    let oc = origin.sub(actor.transform.translation);
                    let b = oc.dot(dir);
                    let r = actor.radius_or_half_extent;
                    let c = oc.dot(oc) - r * r;
                    let disc = b * b - c;
                    if disc >= 0.0 {
                        let root = disc.sqrt();
                        let t0 = -b - root;
                        let t1 = -b + root;
                        if t0 > 0.001 {
                            Some(t0)
                        } else if t1 > 0.001 {
                            Some(t1)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                PrimitiveShape::GroundPlane => {
                    if dir.y.abs() > 1e-5 {
                        let t = (actor.transform.translation.y - origin.y) / dir.y;
                        let hit = origin.add(dir.scale(t));
                        let limit = actor.radius_or_half_extent;
                        if t > 0.001 && hit.x.abs() <= limit && hit.z.abs() <= limit {
                            Some(t)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            };
            if let Some(t) = hit_t
                && best.is_none_or(|(_, prev_t)| t < prev_t)
            {
                best = Some((actor.id, t));
            }
        }
        best
    }
}

/// Decodes a bounded OpenUSD Crate (`usdc`) LZ4 payload block via `craft-usd`.
pub fn decode_usd_crate_frame(compressed: &[u8], max_bytes: usize) -> Result<Vec<u8>> {
    craft_usd::decompress(compressed, max_bytes).context("decoding OpenUSD Crate LZ4 block")
}
