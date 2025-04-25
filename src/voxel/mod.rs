use bevy::pbr::wireframe::WireframeConfig;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::Actions;
use grid::Ordering;
use voxel_grid::{Voxel, VoxelGrid};

use crate::{camera::{ActiveCamera, CameraEntities, CameraToggle, FlyingCamera, FlyingSettings, FlyingState, FollowCamera, FollowSettings, FollowState}, character};

pub mod collider;
pub mod mesh;
pub mod pick;
pub mod raycast;
pub mod voxel_grid;

pub const GRID_SCALE: Vec3 = Vec3::new(1.0, 0.25, 1.0);

/// Flat vec storage of 2d/3d grids.
pub mod grid;

#[derive(Default)]
pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Voxel>(); //.register_type::<VoxelGrid>();

        app.add_plugins(mesh::surface_net::SurfaceNetPlugin);
        app.add_plugins(mesh::ass_mesh::ASSMeshPlugin);
        app.add_plugins(mesh::meshem::BoxMeshPlugin);

        app.add_plugins(pick::VoxelPickPlugin);
        app.add_plugins(collider::VoxelColliderPlugin);
        app.add_plugins(character::plugin);

        app.add_systems(Update, VoxelGrid::clear_changed_system).add_systems(Update, rename_grids);

        app.insert_resource(WireframeConfig { global: false, ..default() });

        app.add_systems(Startup, spawn_voxel_grid);
        app.add_systems(Startup, spawn_directional_lights);
    }
}

pub fn spawn_voxel_grid(mut commands: Commands) {
        // Meshem is XZY
        // Others are XYZ
        let width = 16;
        let length = 16;
        let height = 10;
        let mut grid = VoxelGrid::new([width, height.max(50), length], Ordering::XZY);
        for x in 0..width {
            for z in 0..length {
                for y in 0..height {
                    grid.set([x, y, z], Voxel::Dirt);
                }
            }
        }

        for x in 0..width {
            for z in 0..length {
                for y in (height - 2)..height {
                    grid.set([x, y, z], Voxel::Grass);
                }
            }
        }

        for x in 0..width {
            for z in 0..length {
                for y in 0..1 {
                    grid.set([x, y, z], Voxel::Base);
                }
            }
        }

        commands.spawn((
            grid,
            // mesh::surface_net::SurfaceNet::default(),
            // mesh::ass_mesh::ASSMesh,
            mesh::meshem::Meshem,
        ));


        let flying = commands.spawn((
            Name::new("Flying camera"),
            Actions::<FlyingCamera>::default(),
            FlyingSettings::default(),
            FlyingState::default(),
            Camera { is_active: true, ..default() },
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection::default()),
            Transform::from_translation(Vec3::new(8.0, 10.0, 8.0))
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        )).id();

        let follow = commands.spawn((
            Name::new("Follow camera"),
            Actions::<FollowCamera>::default(),
            FollowSettings::default(),
            FollowState::default(),
            Camera { is_active: false, ..default() },
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection::default()),
            Transform::from_translation(Vec3::new(8.0, 10.0, 8.0))
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        )).id();

        commands.spawn((
            Name::new("Camera toggle"),
            Actions::<CameraToggle>::default(),
            CameraEntities {
                flying: flying,
                follow: follow,
                active: ActiveCamera::Flying,
            }
        ));
}


pub fn spawn_directional_lights(mut commands: Commands) {
    commands.spawn((
        Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)).looking_at(Vec3::ZERO, Vec3::Y),
        DirectionalLight {
            shadows_enabled: false,
            illuminance: 25_000.0,
            color: Color::WHITE,
            ..default()
        },
    ));

    let angled_lights =
        [Vec3::Y + Vec3::Z, Vec3::Y - Vec3::Z, Vec3::Y + Vec3::X, Vec3::Y - Vec3::X];
    for light in angled_lights {
        commands.spawn((
            Transform::from_translation(light).looking_at(Vec3::ZERO, Vec3::Y),
            DirectionalLight {
                shadows_enabled: false,
                illuminance: 10_000.0,
                color: Color::WHITE,
                ..default()
            },
        ));
    }
}

pub fn rename_grids(
    mut commands: Commands,
    grids: Query<Entity, (With<VoxelGrid>, Without<Name>)>,
) {
    for grid in &grids {
        commands.entity(grid).insert(Name::new("Voxel Grid"));
    }
}
