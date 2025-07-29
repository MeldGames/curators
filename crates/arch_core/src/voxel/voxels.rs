use std::cmp::Ordering;
use std::ops::RangeInclusive;

use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
#[cfg(feature = "trace")]
use tracing::*;

use super::raycast::Hit;
use crate::voxel::mesh::binary_greedy::Chunks;
use crate::voxel::mesh::{BinaryGreedy, SurfaceNet, RenderChunks};
use crate::voxel::raycast::VoxelHit;
use crate::voxel::simulation::data::SimChunks;
use crate::voxel::{GRID_SCALE, UpdateVoxelMeshSet, Voxel, VoxelAabb};

pub fn plugin(app: &mut App) {
    // app.add_plugins(super::voxel::plugin);
}

#[derive(Debug, Component, Clone, PartialEq, Eq)]
#[require(Name::new("Voxels"), Transform { scale: GRID_SCALE, ..default() }, SurfaceNet::default(),
    Visibility::Inherited,
    Chunks::default(),
)]
pub struct Voxels {
    // Meshing data
    pub render_chunks: RenderChunks,

    // Simulation data
    pub sim_chunks: SimChunks,

    // Shared data
    pub voxel_size: IVec3,
}

impl Voxels {
    pub fn new(voxel_size: IVec3) -> Self {
        Self {
            render_chunks: RenderChunks::new(voxel_size),
            sim_chunks: SimChunks::new(voxel_size),
            voxel_size,
        }
    }

    #[inline]
    pub fn get_voxel(&self, point: IVec3) -> Voxel {
        self.sim_chunks.get_voxel(point) // sim is source of truth
    }

    #[inline]
    pub fn set_voxel(&mut self, point: IVec3, voxel: Voxel) {
        // self.render_chunks.set_voxel(point, voxel); // is this necessary? the sim
        // chunks should update this later
        self.sim_chunks.set_voxel(point, voxel);
    }

    pub fn voxel_bounds(&self) -> (IVec3, IVec3) {
        // let (min, max) = self.chunk_bounds();
        // (min * unpadded::SIZE as Scalar, max * unpadded::SIZE as Scalar)
        (IVec3::ZERO, self.voxel_size)
    }

    pub fn voxel_aabb(&self) -> VoxelAabb {
        let (min, max) = self.voxel_bounds();
        VoxelAabb::new(min, max)
    }

    /// Raycasting in chunk space.
    /// I don't think this was doing anything for us
    // pub fn chunk_ray_iter(
    //     &self,
    //     grid_transform: &GlobalTransform,
    //     ray: Ray3d,
    //     length: f32,
    // ) -> impl Iterator<Item = Hit> {
    //     const CHUNK_SIZE: Vec3 = Vec3::splat(unpadded::SIZE as f32);
    //     // #[allow(non_snake_case)]
    //     // let SCALED_CHUNK_SIZE: Vec3 = CHUNK_SIZE * crate::voxel::GRID_SCALE;

    //     let inv_matrix = grid_transform.compute_matrix().inverse();
    //     let Ok(local_direction) =
    // Dir3::new(inv_matrix.transform_vector3(ray.direction.as_vec3()))     else
    // {         panic!();
    //     };
    //     let chunk_scaled = Transform { scale: 1.0 / CHUNK_SIZE, ..default() };
    //     let local_origin =
    //         chunk_scaled.compute_matrix().transform_point3(inv_matrix.
    // transform_point3(ray.origin));

    //     let local_ray = Ray3d { origin: local_origin, direction: local_direction
    // };

    //     let (min, max) = self.chunk_bounds();
    //     let volume = VoxelAabb { min, max };
    //     // info!("chunk bounds: {:?}", self.chunk_size());chunk
    //     volume.traverse_ray(local_ray, length)
    // }

    pub fn ray_iter(
        &self,
        grid_transform: &GlobalTransform,
        ray: Ray3d,
        length: f32,
    ) -> impl Iterator<Item = VoxelHit> {
        // translate ray to voxel space
        let local_ray = {
            let inv_matrix = grid_transform.compute_matrix().inverse();
            Ray3d {
                origin: inv_matrix.transform_point3(ray.origin),
                direction: Dir3::new(inv_matrix.transform_vector3(ray.direction.as_vec3()))
                    .unwrap(),
            }
        };

        let volume = VoxelAabb { min: IVec3::ZERO, max: self.voxel_size };
        volume.traverse_ray(local_ray, length).into_iter().map(move |hit| {
            // translate hit back to world space
            let local_distance = hit.distance;
            let local_point = local_ray.origin + local_ray.direction.as_vec3() * local_distance;
            let world_point = grid_transform.transform_point(local_point);
            let world_distance = world_point.distance(ray.origin);
            VoxelHit {
                voxel: hit.voxel,
                world_space: world_point,

                distance: world_distance,
                normal: hit.normal,
            }
        })
    }

    pub fn cast_ray(
        &self,
        grid_transform: &GlobalTransform,
        ray: Ray3d,
        length: f32,
    ) -> Option<VoxelHit> {
        for hit in self.ray_iter(grid_transform, ray, length) {
            // info!("hit: {hit:?}");
            let voxel = self.get_voxel(hit.voxel);
            if voxel.pickable() {
                return Some(hit);
            }
        }
        None
    }

    /// Take a 3x3 matrix around the voxel and smooth it on the y-axis.
    pub fn smooth_voxel(&mut self, pos: IVec3, height_range: RangeInclusive<i32>) {
        #[rustfmt::skip]
        let surrounding = [
            IVec3::new(-1, 0, 1), IVec3::new(0, 0, 1), IVec3::new(1, 0, 1),
            IVec3::new(-1, 0, 0), IVec3::new(0, 0, 0), IVec3::new(1, 0, 0),
            IVec3::new(-1, 0, -1), IVec3::new(0, 0, -1), IVec3::new(1, 0, -1),
        ];

        #[rustfmt::skip]
        let convolution = [
            1, 2, 1,
            2, 3, 2,
            1, 2, 1,
        ];

        let mut heights = [0; 9];
        for (index, relative) in surrounding.iter().enumerate() {
            // find the height by searching up and down until we find [`Voxel::Air`].
            let voxel = pos + relative;

            match self.get_voxel(voxel) {
                Voxel::Air => {
                    while heights[index] > *height_range.start() {
                        match self.get_voxel(voxel + IVec3::Y * heights[index]) {
                            Voxel::Air => {},
                            _ => break,
                        }

                        heights[index] -= 1;
                    }
                },
                _ => {
                    while heights[index] < *height_range.end() {
                        match self.get_voxel(voxel + IVec3::Y * heights[index]) {
                            Voxel::Air => break,
                            _ => {},
                        }

                        heights[index] += 1;
                    }
                },
            };
        }

        let mut sum = 0;
        for index in 0..9 {
            sum += heights[index] * convolution[index];
        }
        let new_height = sum / 9;

        // create a gradient over the range to the current height or the new height if
        // lower.
        let mut gradient = Vec::new();
        for height in *height_range.start()..new_height.max(0) {
            let voxel = self.get_voxel(pos + IVec3::Y * height);
            gradient.push(voxel);
        }

        if new_height < pos.y {
            // squish the voxels down
        } else {
            // stretch the voxels out
        }
    }

    pub fn point_iter<'a, 'b>(&'b self) -> impl Iterator<Item = IVec3> + 'a {
        let voxel_size = self.voxel_size;
        (0..voxel_size.x).flat_map(move |x| {
            (0..voxel_size.z).flat_map(move |z| (0..voxel_size.y).map(move |y| IVec3::new(x, y, z)))
        })
    }

    pub fn diff(&self, other: &Voxels, cutoff: usize) -> Vec<VoxelDiff> {
        let mut diffs = Vec::new();

        for point in self.point_iter() {
            let v1 = self.get_voxel(point);
            let v2 = other.get_voxel(point);
            if v1 != v2 {
                diffs.push(VoxelDiff { point, v1, v2 });
                if diffs.len() > cutoff {
                    break;
                }
            }
        }

        return diffs;
    }
}

#[derive(Debug, Copy, Clone)]
pub struct VoxelDiff {
    pub point: IVec3,
    pub v1: Voxel,
    pub v2: Voxel,
}
