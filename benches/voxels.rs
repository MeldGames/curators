use std::hint::black_box;

use arch::core::sdf::voxel_rasterize::{RasterConfig, RasterVoxel, rasterize};
use arch::core::sdf::{self};
use arch::core::voxel::{self, Voxel, Voxels};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

criterion_group!(benches, get_voxel, set_voxel, updates_iterator);
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

    group.bench_function("set_voxel_sim", |b| {
        let mut voxels = Voxels::new(IVec3::splat(128));

        b.iter(|| {
            for point in voxels.point_iter() {
                black_box(voxels.sim_chunks.set_voxel(point, Voxel::Sand));
            }
        })
    });

    group.bench_function("set_voxel_render", |b| {
        let mut voxels = Voxels::new(IVec3::splat(128));

        b.iter(|| {
            for point in voxels.point_iter() {
                black_box(voxels.render_chunks.set_voxel(point, Voxel::Sand));
            }
        })
    });

    // group.bench_function("set_voxel_sim_by_chunk", |b| {
    //     let mut voxels = Voxels::new(IVec3::splat(128));

    //     b.iter(|| {
    //         for (point, voxel) in point_iter.clone().zip(voxel_iter.clone())
    // {             black_box(voxels.sim_chunks.set_voxel(point, voxel));
    //         }
    //     })
    // });
}

fn updates_iterator(c: &mut Criterion) {
    let mut group = c.benchmark_group("update_iterator");

    group.bench_function("update_set", |b| {
        b.iter_batched(
            || {
                let mut voxels = Voxels::new(IVec3::splat(128));
                voxels.sim_chunks.clear_updates();

                voxels
            },
            |mut voxels| {
                for point in voxels.point_iter() {
                    voxels.sim_chunks.push_point_update(point);
                }

                black_box(&voxels.sim_chunks.sim_updates);
                black_box(&voxels.sim_chunks.render_updates);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("update_iter_dense", |b| {
        b.iter_batched(
            || {
                let voxels = Voxels::new(IVec3::splat(128));
                let swap_buffer = voxels.sim_chunks.create_update_buffer();
                (voxels, swap_buffer)
            },
            |(mut voxels, mut swap_buffer)| {
                for (chunk_index, voxel_index) in voxels.sim_chunks.sim_updates(&mut swap_buffer) {
                    black_box((chunk_index, voxel_index));
                }
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("update_iter_sparse", |b| {
        b.iter_batched(
            || {
                let mut voxels = Voxels::new(IVec3::splat(128));
                voxels.sim_chunks.clear_updates();

                for point in voxels.point_iter().step_by(34) {
                    voxels.sim_chunks.push_point_update(point);
                }

                let swap_buffer = voxels.sim_chunks.create_update_buffer();
                (voxels, swap_buffer)
            },
            |(mut voxels, mut swap_buffer)| {
                for (chunk_index, voxel_index) in voxels.sim_chunks.sim_updates(&mut swap_buffer) {
                    black_box((chunk_index, voxel_index));
                }
            },
            BatchSize::SmallInput,
        );
    });
}
