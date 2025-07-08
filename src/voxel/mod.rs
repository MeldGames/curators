use bevy::pbr::wireframe::WireframeConfig;
use bevy::prelude::*;
use bevy::render::camera::Exposure;
pub use chunk::{VoxelChunk, unpadded, padded, Scalar};
pub use mesh::UpdateVoxelMeshSet;
pub use pick::CursorVoxel;
pub use voxel::Voxel;
pub use voxels::Voxels;
pub use voxel_aabb::VoxelAabb;

use crate::character;

pub mod chunk;
pub mod collider;
pub mod mesh;
pub mod pick;
pub mod raycast;
pub mod voxel;
pub mod voxels;
pub mod voxel_aabb;

// pub const GRID_SCALE: Vec3 = Vec3::new(1.0, 0.2, 1.0);
pub const GRID_SCALE: Vec3 = Vec3::splat(0.35);
// pub const GRID_SCALE: Vec3 = Vec3::new(0.2, 0.2, 0.2);

#[derive(Default)]
pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Voxel>();
        app.register_type::<Exposure>();
        app.add_event::<ChangedChunks>();

        app.add_plugins(pick::VoxelPickPlugin);
        app.add_plugins(collider::plugin)
            .add_plugins(character::plugin)
            .add_plugins(mesh::plugin)
            .add_plugins(raycast::plugin);

        app.insert_resource(WireframeConfig { global: false, ..default() });

        app.add_systems(Startup, spawn_voxel_grid);
        app.add_systems(Startup, spawn_directional_lights);
        app.add_systems(Update, dynamic_scene);
        app.add_systems(PostUpdate, clear_changed_chunks.before(UpdateVoxelMeshSet));
    }
}

pub fn spawn_voxel_grid(mut commands: Commands) {
    commands.spawn((
        Voxels::new(),
        Transform { scale: GRID_SCALE, ..default() },
        // mesh::surface_net::SurfaceNet::default(),
        // mesh::ass_mesh::ASSMesh,
        // mesh::meshem::Meshem,
        mesh::binary_greedy::BinaryGreedy,
    ));
}

#[derive(Event, Debug)]
pub struct ChangedChunks {
    pub voxel_entity: Entity,
    pub changed_chunks: Vec<IVec3>,
}

pub fn clear_changed_chunks(
    mut voxels: Query<(Entity, &mut Voxels)>,
    mut writer: EventWriter<ChangedChunks>,
) {
    for (voxel_entity, mut voxels) in &mut voxels {
        writer.write(ChangedChunks {
            voxel_entity,
            changed_chunks: voxels.changed_chunk_pos_iter().collect::<Vec<_>>(),
        });
        voxels.clear_changed_chunks();
    }
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
