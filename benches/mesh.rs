use arch_core::voxel::mesh::surface_net::fast_surface_nets::SurfaceNetsBuffer;
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
        let mut surface_net_buffer = SurfaceNetsBuffer::default();

        group
            .bench_function(bench.name, |b| {
                b.iter_batched(
                    || {
                        let mut voxels = bench.voxel.new_voxels();
                        bench.voxel.apply_brushes(&mut voxels);
                        let mut swap_buffer = voxels.sim_chunks.create_update_buffer();
                        let Voxels {
                            sim_chunks,
                            render_chunks,
                            ..
                        } = &mut voxels;
                        sim_chunks.propagate_sim_updates(render_chunks, &mut swap_buffer);

                        voxels
                    },
                    |voxels: Voxels| {
                        for (_chunk_pos, chunk) in voxels.render_chunks.chunk_iter() {
                            for voxel in chunk.voxel_type_updates() {
                                if !voxel.rendered() {
                                    continue;
                                }

                                // chunk.update_surface_net_samples(&mut samples.0, voxel.id());
                                chunk.create_surface_net(&mut surface_net_buffer, voxel.id());
                                // for normal in surface_net_buffer.normals.iter_mut() {
                                //     *normal = (Vec3::from(*normal).normalize()).into();
                                // }

                                // let mut mesh = surface_net_to_mesh(&surface_net_buffer);
                                // mesh.duplicate_vertices();
                                // mesh.compute_flat_normals();
                                black_box(&surface_net_buffer);
                            }

                            chunk.update_prev_counts();
                        }
                    },
                    BatchSize::LargeInput,
                );
            })
            .sample_size(bench.measurement.sample_size)
            .measurement_time(bench.measurement.measurement_time);
    }
}
