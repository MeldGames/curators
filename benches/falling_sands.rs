use bevy_math::bounding::Aabb3d;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use bevy::prelude::*;

use arch::{sdf::{
    self,
    voxel_rasterize::{rasterize, RasterConfig, RasterVoxel},
}, voxel::simulation::{data::SimChunks, SimSwapBuffer}};
use arch::voxel::{self, Voxel, Voxels};

criterion_group!(benches, falling_sand_torus);
criterion_main!(benches);

fn falling_sand_torus(c: &mut Criterion) {
    let mut group = c.benchmark_group("falling_sand");
    // group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(10));

    group.bench_function("torus_falling", |b| {
        let mut voxels = Voxels::new(IVec3::new(128, 128, 128));

        // Create a simulation area with a barrier around it.
        let min = 0;
        let max = 60;
        for x in min..max {
            for z in min..max {
                for y in min..max {
                    if x == min || x == max || z == min || z == max || y == min || y == max {
                        voxels.set_voxel(IVec3::new(x, y, z), Voxel::Barrier);
                    }
                }
            }
        }

        b.iter_batched(
            || {
                let mut app = plugin_setup();
                app.world_mut().spawn((voxels.clone(),));
                for _ in 0..5 {
                    app.update();
                } // let settle.

                let mut world = app.world_mut();
                let mut query = world.query::<&mut Voxels>();
                let mut voxels = query.single_mut(&mut world).unwrap();

                let torus = sdf::Torus { minor_radius: 2.0, major_radius: 3.0 };
                for raster_voxel in rasterize(
                    torus,
                    RasterConfig {
                        clip_bounds: Aabb3d::new(Vec3::ZERO, Vec3::splat(100.0)),
                        grid_scale: arch::voxel::GRID_SCALE,
                        pad_bounds: Vec3::ZERO,
                    },
                ) {
                    if raster_voxel.distance <= 0.0 {
                        voxels.set_voxel(raster_voxel.point + IVec3::new(30, 30, 30), Voxel::Sand);
                    }
                }

                // for _ in 0..50 {
                //     app.update();
                // } // let settle.

                app
            },
            |mut app: App| {
                for _ in 0..5 {
                    app.update();
                }
            },
            BatchSize::LargeInput,
        );
    });
}

fn plugin_setup() -> App {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugins(voxel::voxels::plugin)
        .insert_resource(voxel::simulation::FallingSandTick(0))
        .add_systems(Update, voxel::simulation::falling_sands)
        .add_plugins(voxel::simulation::data::plugin);
    app
}
