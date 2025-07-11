use bevy_math::bounding::Aabb3d;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use bevy::prelude::*;

use arch::sdf::{
    self,
    voxel_rasterize::{RasterConfig, RasterVoxel, rasterize},
};
use arch::voxel::{self, Voxel, Voxels};

criterion_group!(benches, get_voxel, set_voxel);
criterion_main!(benches);

fn get_voxel(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_voxels");
    group.bench_function("get_voxel_direct", |b| {
        let voxels = Voxels::new();

        b.iter(|| {
            for y in -10..10 {
                for x in -10..10 {
                    for z in -10..10 {
                        black_box(voxels.get_voxel(IVec3::new(x, y, z)));
                    }
                }
            }
        })
    });
}

fn set_voxel(c: &mut Criterion) {
    let mut group = c.benchmark_group("set_voxels");
    let size = 30;
    let len = size * size * size;
    let point_iter = (-size..size).flat_map(move |y| {
                    (-size..size).flat_map(move |x| (-size..size).map(move |z| IVec3::new(x, y, z)))
                });
    let voxel_iter = (-len..len).map(|_| Voxel::Sand);

    group.bench_function("set_voxel_direct", |b| {
        let mut voxels = Voxels::new();

        b.iter(|| {
            for (point, voxel) in point_iter.clone().zip(voxel_iter.clone()) {
                black_box(voxels.set_voxel(point, voxel));
            }
        })
    });

    group.bench_function("set_voxel_batch", |b| {
        let mut voxels = Voxels::new();

        b.iter(|| {
            black_box(voxels.set_voxels(
                point_iter.clone(),
                voxel_iter.clone(),
            ));
        })
    });
}

fn plugin_setup() -> App {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugins(voxel::voxels::plugin)
        .add_systems(Update, voxel::simulation::falling_sands);
    app
}
