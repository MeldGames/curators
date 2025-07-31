use arch_core::bevy;
use arch_core::sdf::{self, Sdf};
use arch_core::voxel::{self, Voxel};
use bevy::prelude::*;

pub fn plugin_setup() -> App {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugins(voxel::voxels::plugin)
        .insert_resource(voxel::simulation::FallingSandTick(0))
        .add_systems(Update, voxel::simulation::falling_sands)
        .add_plugins(voxel::simulation::data::plugin);
    app
}

pub struct BenchSetup {
    /// Name of the bench setup
    pub name: &'static str,

    pub measurement_time: std::time::Duration,

    pub sample_size: usize,

    /// Size of the voxel grid
    pub voxel_size: IVec3,
    /// How many update iterations to run
    pub test_steps: usize,
    /// Paint voxels in the world each step: (center, brush, voxel)
    pub brushes: Vec<(IVec3, Box<dyn Sdf + Send + Sync>, Voxel)>,
}

pub fn basic_benches() -> Vec<BenchSetup> {
    vec![
        BenchSetup {
            name: "torus_sand",
            measurement_time: std::time::Duration::from_secs(10),
            sample_size: 100,
            voxel_size: IVec3::new(60, 60, 60),
            test_steps: 40,
            brushes: vec![(
                IVec3::new(30, 30, 30),
                Box::new(sdf::Torus { minor_radius: 2.0, major_radius: 3.0 }),
                Voxel::Sand,
            )],
        },
        BenchSetup {
            name: "torus_water",
            measurement_time: std::time::Duration::from_secs(10),
            sample_size: 100,
            voxel_size: IVec3::new(60, 60, 60),
            test_steps: 40,
            brushes: vec![(
                IVec3::new(30, 30, 30),
                Box::new(sdf::Torus { minor_radius: 2.0, major_radius: 3.0 }),
                Voxel::Water { lateral_energy: 32 },
            )],
        },
        BenchSetup {
            name: "sphere_sand_large",
            measurement_time: std::time::Duration::from_secs(40),
            sample_size: 10,
            voxel_size: IVec3::splat(128),
            test_steps: 100,
            brushes: vec![(
                IVec3::splat(128) / 2,
                Box::new(sdf::Sphere { radius: 20.0 }),
                Voxel::Sand,
            )],
        },
        BenchSetup {
            name: "sphere_water_large",
            measurement_time: std::time::Duration::from_secs(40),
            sample_size: 10,
            voxel_size: IVec3::splat(128),
            test_steps: 100,
            brushes: vec![(
                IVec3::splat(128) / 2,
                Box::new(sdf::Sphere { radius: 20.0 }),
                Voxel::Water { lateral_energy: 32 },
            )],
        },
        // BenchSetup {
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
        // BenchSetup {
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
