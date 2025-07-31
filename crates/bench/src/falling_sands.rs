use arch_core::bevy;
use arch_core::sdf::{self, Sdf};
use arch_core::voxel::{self, Voxel};
use bevy::prelude::*;

use crate::{MeasurementSetup, VoxelSetup};

pub fn plugin_setup() -> App {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugins(voxel::voxels::plugin)
        .insert_resource(voxel::simulation::FallingSandTick(0))
        .add_systems(Update, voxel::simulation::falling_sands)
        .add_plugins(voxel::simulation::data::plugin);
    app
}

// Voxel bench setup
pub struct SimBenchSetup {
    /// Name of the bench setup
    pub name: &'static str,
    /// How many update iterations to run
    pub test_steps: usize,

    pub measurement: MeasurementSetup,
    pub voxel: VoxelSetup,
}

impl Default for SimBenchSetup {
    fn default() -> Self {
        Self {
            name: "unnamed bench",
            test_steps: 50,
            measurement: MeasurementSetup::default(),
            voxel: VoxelSetup::default(),
        }
    }
}

pub fn basic_benches() -> Vec<SimBenchSetup> {
    vec![
        SimBenchSetup {
            name: "torus_sand",
            test_steps: 40,
            measurement: MeasurementSetup::default(),
            voxel: VoxelSetup {
                voxel_size: IVec3::new(60, 60, 60),
                brushes: vec![(
                    IVec3::new(30, 30, 30),
                    Box::new(sdf::Torus { minor_radius: 2.0, major_radius: 3.0 }),
                    Voxel::Sand,
                )],
            }
        },
        SimBenchSetup {
            name: "torus_water",
            test_steps: 40,
            measurement: MeasurementSetup::default(),
            voxel: VoxelSetup {
                voxel_size: IVec3::new(60, 60, 60),
                brushes: vec![(
                    IVec3::new(30, 30, 30),
                    Box::new(sdf::Torus { minor_radius: 2.0, major_radius: 3.0 }),
                    Voxel::Water { lateral_energy: 32 },
                )],
            }
        },
        SimBenchSetup {
            name: "sphere_sand_large",
            test_steps: 100,
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
            }
        },
        SimBenchSetup {
            name: "sphere_water_large",
            test_steps: 100,
            measurement: MeasurementSetup {
                measurement_time: std::time::Duration::from_secs(40),
                sample_size: 10,
            },
            voxel: VoxelSetup {
                voxel_size: IVec3::splat(128),
                brushes: vec![(
                    IVec3::splat(128) / 2,
                    Box::new(sdf::Sphere { radius: 20.0 }),
                    Voxel::Water { lateral_energy: 32 },
                )],
            }
        },
        // SimBenchSetup {
        //     name: "blob",
        //     voxel_size: IVec3::splat(256),
        //     test_steps: 2000,
        //     brushes: vec![(
        //         IVec3::splat(256) / 2,
        //         Box::new(sdf::ops::Scale {
        //             scale: Vec3::splat(15.0),
        //             primitive: sdf::Blob,
        //         }),
        //         Voxel::Sand,
        //     )],
        // },
        // SimBenchSetup {
        //     name: "fractal",
        //     voxel_size: IVec3::splat(256),
        //     test_steps: 2000,
        //     brushes: vec![(
        //         IVec3::splat(256) / 2,
        //         // Box::new(sdf::Fractal),
        //         Box::new(sdf::ops::Scale {
        //             scale: Vec3::splat(0.0001),
        //             primitive: sdf::Fractal,
        //         }),
        //         Voxel::Base,
        //     )],
        // },
    ]
}
