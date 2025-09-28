use arch_core::bevy;
use arch_core::bevy::math::bounding::Aabb3d;
use arch_core::bevy::render::RenderPlugin;
use arch_core::sdf::{self, Sdf};
use arch_core::voxel::simulation::SimSettings;
use arch_core::voxel::{self, GRID_SCALE, Voxel, Voxels};
use bevy::prelude::*;

use crate::{MeasurementSetup, VoxelSetup};

pub fn bench_setup(app: &mut App) {
    app
        .add_plugins(voxel::voxels::plugin)
        .add_plugins(voxel::voxel::plugin)
        // .add_systems(PostUpdate, update_render_voxels)
        .add_plugins(voxel::mesh::plugin)
        .add_plugins(voxel::simulation::data::plugin);
}

pub struct MeshBenchSetup {
    pub name: &'static str,
    pub measurement: MeasurementSetup,
    pub voxel: VoxelSetup,
}

pub fn mesh_benches() -> Vec<MeshBenchSetup> {
    vec![
        MeshBenchSetup {
            name: "torus_sand",
            measurement: MeasurementSetup::default(),
            voxel: VoxelSetup {
                voxel_size: IVec3::new(60, 60, 60),
                brushes: vec![(
                    IVec3::new(30, 30, 30),
                    Box::new(sdf::Torus { minor_radius: 2.0, major_radius: 3.0 }),
                    Voxel::Sand,
                )],
            },
        },
        MeshBenchSetup {
            name: "torus_water",
            measurement: MeasurementSetup::default(),
            voxel: VoxelSetup {
                voxel_size: IVec3::new(60, 60, 60),
                brushes: vec![(
                    IVec3::new(30, 30, 30),
                    Box::new(sdf::Torus { minor_radius: 2.0, major_radius: 3.0 }),
                    Voxel::Water(default()),
                )],
            },
        },
        MeshBenchSetup {
            name: "sphere_sand_large",
            measurement: MeasurementSetup {
                measurement_time: std::time::Duration::from_secs(40),
                sample_size: 10,
            },
            voxel: VoxelSetup {
                voxel_size: IVec3::splat(128),
                brushes: vec![(
                    IVec3::splat(128) / 2,
                    Box::new(sdf::Sphere { radius: 20.0 }),
                    Voxel::Sand,
                )],
            },
        },
        MeshBenchSetup {
            name: "sphere_water_large",
            measurement: MeasurementSetup {
                measurement_time: std::time::Duration::from_secs(40),
                sample_size: 10,
            },
            voxel: VoxelSetup {
                voxel_size: IVec3::splat(128),
                brushes: vec![(
                    IVec3::splat(128) / 2,
                    Box::new(sdf::Sphere { radius: 20.0 }),
                    Voxel::Water(default()),
                )],
            },
        },
    ]
}
