use bevy::pbr::wireframe::WireframeConfig;
use bevy::prelude::*;
use grid::Ordering;
use voxel_grid::{Voxel, VoxelGrid};

pub mod collider;
pub mod mesh;
pub mod pick;
pub mod raycast;
pub mod voxel_grid;

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

        app.add_systems(Update, VoxelGrid::clear_changed_system).add_systems(Update, rename_grids);

        app.insert_resource(WireframeConfig { global: false, ..default() });

        // Meshem is XZY
        // Others are XYZ
        let width = 64;
        let length = 64;
        let height = 20;
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

        for x in 0..4 {
            for z in 0..4 {
                grid.set([x, 1, z], Voxel::Dirt);
            }
        }

        // for x in 0..3 {
        // for z in 0..3 {
        // grid.set([x, 2, z], Voxel::Dirt);
        // }
        // }

        for x in 0..2 {
            grid.set([x, 2, 0], Voxel::Dirt);
            grid.set([0, 2, x], Voxel::Dirt);
        }

        grid.set([1, 2, 1], Voxel::Dirt);
        grid.set([2, 2, 2], Voxel::Dirt);

        for y in 3..=5 {
            grid.set([0, y, 0], Voxel::Dirt);
        }

        for y in 0..=8 {
            grid.set([4, y, 0], Voxel::Dirt);
        }

        grid.set([8, 2, 1], Voxel::Dirt);
        grid.set([8, 2, 3], Voxel::Dirt);
        grid.set([8, 2, 5], Voxel::Dirt);
        grid.set([7, 2, 4], Voxel::Dirt);

        grid.set([1, 1, 1], Voxel::Dirt);

        app.world_mut().spawn((
            grid,
            // mesh::surface_net::SurfaceNet::default(),
            // mesh::ass_mesh::ASSMesh,
            mesh::meshem::Meshem,
        ));

        app.add_systems(Startup, spawn_directional_lights);
        // app.world_mut().spawn((
        // Transform::from_translation(Vec3::new(3.0, 3.0, 3.0)),
        // PointLight { range: 200.0, intensity: 800000.0, shadows_enabled: true,
        // ..Default::default() }, ));

        app.world_mut().spawn((
            crate::camera::CameraController::default(),
            Camera { is_active: true, ..default() },
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection::default()),
            Transform::from_translation(Vec3::new(8.0, 10.0, 8.0))
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ));
    }
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
