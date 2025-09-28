use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use bevy::prelude::*;

use arch::core::voxel::{self, Voxel, Voxels};
use arch::core::{
    sdf::{
        self,
        voxel_rasterize::{RasterConfig, RasterVoxel, rasterize},
    },
    voxel::simulation::data::SimChunks,
};
use bench::falling_sands::{SimBenchSetup, plugin_setup};

criterion_group!(benches, falling_sand);
criterion_main!(benches);

fn falling_sand(c: &mut Criterion) {
    let mut group = c.benchmark_group("falling_sand");

    for bench in bench::falling_sands::basic_benches() {
        group
            .bench_function(bench.name, |b| {
                b.iter_batched(
                    || {
                        let mut app = plugin_setup();
                        app.world_mut().spawn(bench.voxel.new_sim());
                        app.update(); // initialization stuffs

                        let world = app.world_mut();
                        let mut query = world.query::<&mut SimChunks>();
                        let mut sim_chunks = query.single_mut(world).unwrap();

                        bench.voxel.apply_brushes_sim(&mut sim_chunks);
                        app
                    },
                    |mut app: App| {
                        for _ in 0..bench.test_steps {
                            app.update();
                        }

                        black_box(app);
                    },
                    BatchSize::LargeInput,
                );
            })
            .sample_size(bench.measurement.sample_size)
            .measurement_time(bench.measurement.measurement_time);
    }
}
