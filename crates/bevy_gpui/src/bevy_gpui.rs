//! Aphrody's Unreal Engine 5.8-class 3D/2D studio engine (`bevy_gpui`) integrated natively into Zed:
//! - [`ecs_world`]: Archetype ECS world, actors, PBR materials, cameras, ray picking, USD Crate LZ4 & Mocap tracks.
//! - [`virtual_geometry`]: Nanite-inspired 64-vertex / 124-triangle Meshlet cluster builder, cone & frustum culler.
//! - [`material_solari`]: `craft-materialx` OpenPBR/StandardSurface/glTF -> Naga v30 validated WGSL + Solari GI uniforms.
//! - [`viewport`]: GGX PBR + Solari Sky-IBL + Soft-Shadow + Nanite Meshlet Heatmap renderer & `bevy_egui` 0.37 inspector.

pub mod ecs_world;
pub mod material_solari;
pub mod viewport;
pub mod virtual_geometry;

pub use craft_materialx;
pub use craft_timeline;
pub use craft_usd;

pub use ecs_world::{
    ActorEntity, ActorId, CameraRig, DirectionalSunLight, MocapKeyframe, PbrSurfaceMaterial,
    PrimitiveShape, Transform3d, UnrealWorld, Vec3f, decode_usd_crate_frame,
};
pub use material_solari::{
    CompiledMaterialShader, SolariGiSettings, compile_materialx_to_wgsl,
    default_openpbr_materialx_xml,
};
pub use viewport::{
    BevyEguiInspectorSnapshot, ViewportRenderStats, ViewportShadingMode,
    render_bevy_egui_inspector, render_viewport_rgba8,
};
pub use virtual_geometry::{
    MeshAsset, MeshVertex, MeshletCluster, MeshletCullingStats, build_meshlets, build_uv_sphere,
    cull_meshlets,
};
