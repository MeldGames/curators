use bevy::math::bounding::Aabb3d;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use bevy::prelude::*;

use arch::core::voxel::{self, Voxel, Voxels};
use arch::core::{
    sdf::{
        self,
        voxel_rasterize::{RasterConfig, RasterVoxel, rasterize},
    },
    voxel::simulation::{SimSwapBuffer, data::SimChunks},
};
use bench::falling_sands::{BenchSetup, paint_brush, plugin_setup};

criterion_group!(benches, surface_nets);
criterion_main!(benches);

fn surface_nets(c: &mut Criterion) {
    let mut group = c.benchmark_group("surface_nets");
    // group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(10));

    for bench in bench::falling_sands::basic_benches() {
        group
            .bench_function(bench.name, |b| {
                b.iter_batched(
                    || {
                        let mut app = plugin_setup();
                        app.world_mut().spawn(Voxels::new(bench.voxel_size));
                        app.update(); // initialization stuffs

                        let world = app.world_mut();
                        let mut query = world.query::<&mut Voxels>();
                        let mut voxels = query.single_mut(world).unwrap();

                        for (center, brush, voxel) in &bench.brushes {
                            paint_brush(&mut *voxels, *center, &**brush, *voxel);
                        }

                        app
                    },
                    |mut app: App| {
                        for _ in 0..bench.test_steps {
                            app.update();
                        }
                    },
                    BatchSize::LargeInput,
                );
            })
            .sample_size(bench.sample_size)
            .measurement_time(bench.measurement_time);
    }
}
