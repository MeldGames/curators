use bevy::pbr::light_consts::lux;
use bevy::pbr::wireframe::WireframeConfig;
use bevy::prelude::*;
use bevy::render::camera::Exposure;

pub use chunk::VoxelChunk;
pub use voxel::Voxel;
pub use mesh::UpdateVoxelMeshSet;

use crate::character;

pub mod collider;
pub mod mesh;
pub mod pick;
pub mod raycast;
pub mod chunk;
pub mod voxel;

pub const GRID_SCALE: Vec3 = Vec3::new(1.0, 0.2, 1.0);
//pub const GRID_SCALE: Vec3 = Vec3::new(0.2, 0.2, 0.2);

#[derive(Default)]
pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Voxel>();
        app.register_type::<Exposure>();

        app.add_plugins(pick::VoxelPickPlugin);
        app.add_plugins(collider::plugin)
            .add_plugins(character::plugin)
            .add_plugins(mesh::plugin);

        app.add_systems(Update, VoxelChunk::clear_changed_system);

        app.insert_resource(WireframeConfig { global: false, ..default() });

        app.add_systems(Startup, spawn_voxel_grid);
        app.add_systems(Startup, spawn_directional_lights);
        app.add_systems(Update, dynamic_scene);
    }
}

pub fn spawn_voxel_grid(mut commands: Commands) {
    let mut grid = VoxelChunk::new();
    
    let width = grid.x_size();
    let length = grid.z_size();
    let height = grid.y_size();

    // let width = 16;
    // let length = 16;
    // let height = 31;
    let ground_level = grid.ground_level();
    for x in 0..width {
        for z in 0..length {
            for y in 0..ground_level {
                grid.set([x, y, z], Voxel::Dirt.into());
            }
        }
    }

    for x in 0..width {
        for z in 0..length {
            for y in (ground_level - 2)..ground_level {
                grid.set([x, y, z], Voxel::Grass.into());
            }
        }
    }

    for x in 0..width {
        for z in 0..length {
            for y in 0..1 {
                grid.set([x, y, z], Voxel::Base.into());
            }
        }
    }

    commands.spawn((
        grid,
        Transform { scale: GRID_SCALE, ..default() },
        // mesh::surface_net::SurfaceNet::default(),
        // mesh::ass_mesh::ASSMesh,
        // mesh::meshem::Meshem,
        mesh::binary_greedy::BinaryGreedy,
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
            illuminance: lux::RAW_SUNLIGHT,
            ..default()
        },
        Sun,
    ));

    commands.spawn((Transform::from_xyz(5.0, 5.0, 5.0), PointLight {
        color: Color::srgb(1.0, 0.0, 0.0),
        intensity: 900_000.0,
        range: 100.0,
        radius: 10.0,
        shadows_enabled: true,
        ..default()
    }));

    commands.spawn((
        Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        SpotLight {
            color: Color::srgb(0.0, 1.0, 1.0),
            intensity: 100_000_000.0,
            range: 100.0,
            shadows_enabled: true,
            ..default()
        },
    ));

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
