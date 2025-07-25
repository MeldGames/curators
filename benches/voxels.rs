use bevy::math::bounding::Aabb3d;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use bevy::prelude::*;

use arch::core::sdf::{
    self,
    voxel_rasterize::{RasterConfig, RasterVoxel, rasterize},
};
use arch::core::voxel::{self, Voxel, Voxels};

criterion_group!(benches, get_voxel, set_voxel);
criterion_main!(benches);

fn get_voxel(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_voxels");
    group.bench_function("get_voxel_sim", |b| {
        let voxels = Voxels::new(IVec3::splat(128));

        b.iter(|| {
            for point in voxels.point_iter() {
                black_box(voxels.sim_chunks.get_voxel(point));
            }
        })
    });

    group.bench_function("get_voxel_render", |b| {
        let voxels = Voxels::new(IVec3::splat(128));

        b.iter(|| {
            for point in voxels.point_iter() {
                black_box(voxels.render_chunks.get_voxel(point));
            }
        })
    });
}

fn set_voxel(c: &mut Criterion) {
    let mut group = c.benchmark_group("set_voxels");
    let size = 16;
    let len = size * size * size;
    let point_iter = (0..size).flat_map(move |y| {
        (0..size).flat_map(move |x| (0..size).map(move |z| IVec3::new(x, y, z)))
    });
    let voxel_iter = (0..len).map(|_| Voxel::Sand);

    group.bench_function("set_voxel_sim", |b| {
        let mut voxels = Voxels::new(IVec3::splat(128));

        b.iter(|| {
            for (point, voxel) in point_iter.clone().zip(voxel_iter.clone()) {
                black_box(voxels.sim_chunks.set_voxel(point, voxel));
            }
        })
    });

    group.bench_function("set_voxel_render", |b| {
        let mut voxels = Voxels::new(IVec3::splat(128));

        b.iter(|| {
            for (point, voxel) in point_iter.clone().zip(voxel_iter.clone()) {
                black_box(voxels.render_chunks.set_voxel(point, voxel));
            }
        })
    });

    // group.bench_function("set_voxel_sim_by_chunk", |b| {
    //     let mut voxels = Voxels::new(IVec3::splat(128));

    //     b.iter(|| {
    //         for (point, voxel) in point_iter.clone().zip(voxel_iter.clone()) {
    //             black_box(voxels.sim_chunks.set_voxel(point, voxel));
    //         }
    //     })
    // });
}
