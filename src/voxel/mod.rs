use bevy::{pbr::wireframe::WireframeConfig, prelude::*};
use grid::Ordering;
use voxel_grid::{Voxel, VoxelGrid};

pub mod pick;
pub mod raycast;
pub mod voxel_grid;
pub mod box_mesh;
pub mod ass_mesh;
pub mod surface_net;

/// Flat vec storage of 2d/3d grids.
pub mod grid;

#[derive(Default)]
pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Voxel>();//.register_type::<VoxelGrid>();

        app.add_plugins(surface_net::SurfaceNetPlugin);
        app.add_plugins(ass_mesh::ASSMeshPlugin);
        app.add_plugins(box_mesh::BoxMeshPlugin);

        app.add_plugins(pick::VoxelPickPlugin);

        app.insert_resource(WireframeConfig { global: true, ..default() });

        // Meshem is XZY
        // Others are XYZ
        let mut grid = VoxelGrid::new([50, 50, 50], Ordering::XZY);
        for x in 0..4 {
            for z in 0..4 {
                grid.set([x, 1, z], Voxel::Dirt);
            }
        }

        /*for x in 0..3 {
            for z in 0..3 {
                grid.set([x, 2, z], Voxel::Dirt);
            }
        }*/

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
            //surface_net::SurfaceNet::default(),
            //ass_mesh::ASSMesh,
            box_mesh::Meshem,
        ));

        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(4.0, 9.0, 4.0)),
            PointLight { range: 200.0, intensity: 800000.0, ..Default::default() },
        ));
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
