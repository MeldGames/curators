use bevy::prelude::*;
pub use commands::VoxelCommand;
pub use mesh::UpdateVoxelMeshSet;
pub use pick::CursorVoxel;
pub use tree::{VoxelNode, VoxelTree};
pub use voxel::{Voxel, VoxelSet};
pub use voxel_aabb::VoxelAabb;
pub use voxels::Voxels;

use crate::voxel::commands::VoxelCommandQueue;
use crate::voxel::simulation::data::SimChunks;

pub mod brush;
pub mod collider;
pub mod commands;
pub mod mesh;
pub mod painter;
pub mod pick;
pub mod raycast;
pub mod simulation;
pub mod tree;
pub mod voxel;
pub mod voxel_aabb;
pub mod voxels;

// pub const GRID_SCALE: Vec3 = Vec3::new(1.0, 0.2, 1.0);
pub const GRID_SCALE: Vec3 = Vec3::splat(0.2);
// pub const GRID_SCALE: Vec3 = Vec3::splat(1.0);
// pub const GRID_SCALE: Vec3 = Vec3::new(0.2, 0.2, 0.2);

#[derive(Default)]
pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(pick::VoxelPickPlugin)
            .add_plugins(voxel::plugin)
            .add_plugins(voxels::plugin)
            .add_plugins(tree::plugin)
            .add_plugins(collider::plugin)
            .add_plugins(commands::plugin)
            .add_plugins(mesh::plugin)
            .add_plugins(raycast::plugin)
            .add_plugins(painter::plugin)
            .add_plugins(simulation::plugin);

        app.add_systems(Startup, spawn_voxel_grid);
        app.add_systems(Startup, spawn_directional_lights);
        app.add_systems(Update, dynamic_scene);
    }
}

pub fn spawn_voxel_grid(mut commands: Commands) {
    commands.spawn((
        Voxels::new(IVec3::new(1000, 1000, 1000)),
        // Voxels::new(IVec3::new(15, 15, 15)),
        Transform { scale: GRID_SCALE, ..default() },
        SimChunks::new(),
        VoxelCommandQueue::default(),
        mesh::surface_net::SurfaceNet::default(),
        // mesh::ass_mesh::ASSMesh,
        // mesh::meshem::Meshem,
        // mesh::binary_greedy::BinaryGreedy,
    ));
}
fn dynamic_scene(mut suns: Query<&mut Transform, With<Sun>>, time: Res<Time>) {
    suns.iter_mut()
        .for_each(|mut tf| tf.rotate_z(-time.delta_secs() * std::f32::consts::PI / 1000.0));
}

#[derive(Component)]
pub struct Sun;

pub fn spawn_directional_lights(mut commands: Commands) {
    commands.spawn((
        Transform::from_translation(Vec3::new(0.0, 1.0, 0.5)).looking_at(Vec3::ZERO, Vec3::Y),
        DirectionalLight {
            shadows_enabled: true,
            soft_shadow_size: Some(1.0),
            // illuminance: lux::RAW_SUNLIGHT,
            ..default()
        },
        Sun,
    ));

    // commands.spawn((Transform::from_xyz(5.0, 5.0, 5.0), PointLight {
    //     color: Color::srgb(1.0, 0.0, 0.0),
    //     intensity: 900_000.0,
    //     range: 100.0,
    //     radius: 10.0,
    //     shadows_enabled: true,
    //     ..default()
    // }));

    // commands.spawn((
    //     Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    //     SpotLight {
    //         color: Color::srgb(0.0, 1.0, 1.0),
    //         intensity: 100_000_000.0,
    //         range: 100.0,
    //         shadows_enabled: true,
    //         ..default()
    //     },
    // ));

    // let steepness = 3.0;
    // let height = Vec3::Y * steepness;
    // let height = 0.0;
    //
    // let angled_lights =
    // [height + Vec3::Z, height - Vec3::Z, height + Vec3::X, height - Vec3::X];
    // for light in angled_lights {
    // commands.spawn((
    // Transform::from_translation(light).looking_at(Vec3::ZERO, Vec3::Y),
    // DirectionalLight {
    // shadows_enabled: false,
    // illuminance: 50_000.0,
    // color: Color::WHITE,
    // ..default()
    // },
    // ));
    // }
}
