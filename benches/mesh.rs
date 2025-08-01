use arch_core::voxel::mesh::SurfaceNet;
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
use bench::surface_net::{bench_setup};

criterion_group!(benches, meshing);
criterion_main!(benches);

fn meshing(c: &mut Criterion) {
    let mut group = c.benchmark_group("surface_net");

    for bench in bench::surface_net::mesh_benches() {
        group
            .bench_function(bench.name, |b| {
                b.iter_batched(
                    || {
                        let mut app = App::new();
                        app.add_plugins(MinimalPlugins);
                        app.add_plugins(AssetPlugin::default());
                        app.insert_resource(Assets::<Mesh>::default());
                        app.insert_resource(Assets::<StandardMaterial>::default());
                        app.add_plugins(bench_setup);
                        app.world_mut().spawn((bench.voxel.new_voxels(), SurfaceNet));
                        app.update(); // initialization stuffs

                        let world = app.world_mut();
                        let mut query = world.query::<&mut Voxels>();
                        let mut voxels = query.single_mut(world).unwrap();

                        bench.voxel.apply_brushes(&mut voxels);
                        app
                    },
                    |mut app: App| {
                        app.update();
                        app.update();
                        app.update();
                        black_box(app);
                    },
                    BatchSize::LargeInput,
                );
            })
            .sample_size(bench.measurement.sample_size)
            .measurement_time(bench.measurement.measurement_time);
    }
}
