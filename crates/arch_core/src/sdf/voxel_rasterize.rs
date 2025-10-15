use std::sync::Arc;

use bevy::prelude::*;
use bevy_math::bounding::{Aabb3d, BoundingVolume};

use crate::{sdf::{ops, Sdf}, voxel::data::ChunkPoint};

#[derive(Debug, Copy, Clone)]
pub struct RasterVoxel {
    pub point: IVec3,
    pub distance: f32,
}

#[derive(Debug, Clone)]
pub struct RasterConfig {
    pub clip_bounds: Aabb3d,
    pub grid_scale: Vec3,
    pub pad_bounds: Vec3,
}

#[derive(Debug)]
pub struct RasterIterator<S: Sdf, I: Iterator<Item = IVec3>> {
    sdf: S,
    sample_points: I,
}

impl<S: Sdf, I: Iterator<Item = IVec3>> Iterator for RasterIterator<S, I> {
    type Item = RasterVoxel;

    fn next(&mut self) -> Option<Self::Item> {
        let sample_point = self.sample_points.next()?;
        let distance = self.sdf.sdf(sample_point.as_vec3());
        Some(RasterVoxel { point: sample_point, distance })
    }
}

#[derive(Debug)]
pub struct RasterChunkIterator<S: Sdf, I: Iterator<Item = IVec3>> {
    sdf: S,
    min: IVec3,
    max: IVec3,
    chunk_points: I,
    chunk_width: i32,
}

impl<S: Sdf + Clone, I: Iterator<Item = IVec3>> Iterator for RasterChunkIterator<S, I> {
    type Item = (ChunkPoint, RasterIterator<S, ClampedSamplesIter>);

    fn next(&mut self) -> Option<Self::Item> {
        let chunk_point = self.chunk_points.next()?;
        let iterator = RasterIterator {
            sdf: self.sdf.clone(),
            sample_points: ClampedSamplesIter::new(self.min, self.max, chunk_point, self.chunk_width),
        };
        Some((ChunkPoint(chunk_point), iterator))
    }
}

// Clamp local sample points of a chunk
#[derive(Debug, Clone)]
pub struct ClampedSamplesIter {
    min: IVec3,
    max: IVec3,

    current: i32,
    length: i32,
    bounds: IVec3,
}

impl ClampedSamplesIter {
    pub fn new(world_min: IVec3, world_max: IVec3, chunk_point: IVec3, chunk_width: i32) -> Self {
        // chunk point in voxel world space
        let min_chunk_point = chunk_point * chunk_width;
        // let max_chunk_point = min_chunk_point + IVec3::splat(chunk_width);

        // get the relative world space points
        let relative_min = world_min - min_chunk_point;
        let relative_max = world_max - min_chunk_point;

        println!("relative: {:?}..{:?}", relative_min, relative_max);

        let min = relative_min.clamp(IVec3::ZERO, IVec3::splat(chunk_width));
        let max = relative_max.clamp(IVec3::ZERO, IVec3::splat(chunk_width));

        let bounds = max - min;
        let length = bounds.x * bounds.y * bounds.z;

        Self {
            min: min,
            max: max,

            current: 0,
            length: length,
            bounds: bounds,
        }
    }
}

impl Iterator for ClampedSamplesIter {
    type Item = IVec3;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.length {
            return None;
        } 

        // delinearize point
        let y = self.current / (self.bounds.x * self.bounds.z);
        let x = (self.current / self.bounds.z) % self.bounds.z;
        let z = self.current % self.bounds.z;

        let next_point = self.min + ivec3(x as i32, y as i32, z as i32);
        self.current += 1;
        Some(next_point)
    }
}

pub fn rasterize<S: Sdf>(
    sdf: S,
    config: RasterConfig,
) -> RasterIterator<ops::Scale<S>, impl Iterator<Item = IVec3>> {
    let aabb = sdf
        .aabb()
        .map(|aabb| aabb.grow(Vec3A::from(config.pad_bounds)))
        .map(|aabb| Aabb3d {
            min: aabb.min.max(config.clip_bounds.min),
            max: aabb.max.min(config.clip_bounds.max),
        })
        .unwrap_or(config.clip_bounds);

    let min = (Vec3::from(aabb.min) / config.grid_scale).floor().as_ivec3();
    let max = (Vec3::from(aabb.max) / config.grid_scale).ceil().as_ivec3();

    let point_iter = (min.x..max.x).flat_map(move |x| {
        (min.y..max.y).flat_map(move |y| (min.z..max.z).map(move |z| IVec3::new(x, y, z)))
    });

    RasterIterator {
        sdf: ops::Scale { primitive: sdf, scale: 1.0 / config.grid_scale }, /* might need to invert this scale? */
        sample_points: point_iter,
        // config,
    }
}

pub fn rasterize_chunkwise<S: Sdf>(
    sdf: S,
    config: RasterConfig,
    chunk_width: i32, 
) -> RasterIterator<ops::Scale<S>, impl Iterator<Item = IVec3>> {
    let aabb = sdf
        .aabb()
        .map(|aabb| aabb.grow(Vec3A::from(config.pad_bounds)))
        .map(|aabb| Aabb3d {
            min: aabb.min.max(config.clip_bounds.min),
            max: aabb.max.min(config.clip_bounds.max),
        })
        .unwrap_or(config.clip_bounds);

    let min = (Vec3::from(aabb.min) / config.grid_scale).floor().as_ivec3();
    let max = (Vec3::from(aabb.max) / config.grid_scale).ceil().as_ivec3();

    let min_chunk = min / chunk_width;
    let max_chunk = max / chunk_width;

    let chunk_point_iter = (min_chunk.x..max_chunk.x).flat_map(move |x| {
        (min_chunk.y..max_chunk.y).flat_map(move |y| (min_chunk.z..max_chunk.z).map(move |z| IVec3::new(x, y, z)))
    });

    // 
    // iterate over points in each chunk individually to keep cache locality

    let point_iter = (min.x..max.x).flat_map(move |x| {
        (min.y..max.y).flat_map(move |y| (min.z..max.z).map(move |z| IVec3::new(x, y, z)))
    });

    RasterIterator {
        sdf: ops::Scale { primitive: sdf, scale: 1.0 / config.grid_scale }, /* might need to invert this scale? */
        sample_points: point_iter,
        // config,
    }
}


#[cfg(test)]
mod test {
    use crate::sdf::{self, Sdf, voxel_rasterize::{ClampedSamplesIter, RasterChunkIterator}};
    use bevy::prelude::*;

    #[test]
    fn clamped_samples_iter() {
        let chunk_width = 16;

        // basic origin samples, no clipping
        let min = IVec3::splat(0);
        let max = IVec3::splat(16);
        let mut samples_iter = ClampedSamplesIter::new(min, max, IVec3::new(0, 0, 0), chunk_width);

        let valid_points = (min.y..max.y).flat_map(move |y| {
            (min.x..max.x).flat_map(move |x| (min.z..max.z).map(move |z| IVec3::new(x, y, z)))
        }).collect::<Vec<_>>();

        let mut index = 0;
        while let Some(point) = samples_iter.next() {
            assert_eq!(valid_points[index], point);
            index += 1;
        }
        assert_eq!(index, valid_points.len());

        // origin samples, clipping min/max
        let min = IVec3::splat(2);
        let max = IVec3::splat(10);
        let mut samples_iter = ClampedSamplesIter::new(min, max, IVec3::new(0, 0, 0), chunk_width);

        let valid_points = (min.y..max.y).flat_map(move |y| {
            (min.x..max.x).flat_map(move |x| (min.z..max.z).map(move |z| IVec3::new(x, y, z)))
        }).collect::<Vec<_>>();

        let mut index = 0;
        while let Some(point) = samples_iter.next() {
            assert_eq!(valid_points[index], point);
            index += 1;
        }
        assert_eq!(index, valid_points.len());

        // oob samples
        let min = IVec3::splat(2);
        let max = IVec3::splat(10);
        let mut samples_iter = ClampedSamplesIter::new(min, max, IVec3::new(1, 0, 0), chunk_width);
        assert_eq!(samples_iter.next(), None);

        // oob max, in bound min
        let min = IVec3::splat(2);
        let max = IVec3::splat(30);
        let mut samples_iter = ClampedSamplesIter::new(min, max, IVec3::new(0, 0, 0), chunk_width);

        let valid_points = (min.y..chunk_width).flat_map(move |y| {
            (min.x..chunk_width).flat_map(move |x| (min.z..chunk_width).map(move |z| IVec3::new(x, y, z)))
        }).collect::<Vec<_>>();

        let mut index = 0;
        while let Some(point) = samples_iter.next() {
            assert_eq!(valid_points[index], point);
            index += 1;
        }
        assert_eq!(index, valid_points.len());

        // in bounds max, oob in
        let min = IVec3::splat(2);
        let max = IVec3::splat(30);
        let mut samples_iter = ClampedSamplesIter::new(min, max, IVec3::new(1, 1, 1), chunk_width);

        // 30 - 16 = 14
        let valid_points = (0..14).flat_map(move |y| {
            (0..14).flat_map(move |x| (0..14).map(move |z| IVec3::new(x, y, z)))
        }).collect::<Vec<_>>();

        let mut index = 0;
        while let Some(point) = samples_iter.next() {
            assert_eq!(valid_points[index], point);
            index += 1;
        }
        assert_eq!(index, valid_points.len());
    }

    #[test]
    fn chunk_rasterize_iter() {
        let chunk_width = 16;
        let radius = 8.0;
        let half_size = Vec3::new(15.0, 31.0, 15.0);
        let sdf = sdf::Cuboid {
            half_size: half_size,
        }.translate(half_size);
        // let sdf = sdf::Sphere {
        //     radius: radius, 
        // }.translate(Vec3::splat(radius));

        let aabb = sdf.aabb().unwrap();

        let min = aabb.min.as_ivec3();
        let max = aabb.max.as_ivec3();
        let chunk_min = min / chunk_width;
        let chunk_max = (max - IVec3::ONE) / chunk_width;

        let chunk_points = (chunk_min.y..=chunk_max.y).flat_map(move |y| {
            (chunk_min.x..=chunk_max.x).flat_map(move |x| (chunk_min.z..=chunk_max.z).map(move |z| IVec3::new(x, y, z)))
        });

        let mut iter = RasterChunkIterator {
            sdf: sdf::Sphere {
                radius: 1.0,
            },
            min: min,
            max: max,
            chunk_points: chunk_points,
            chunk_width: chunk_width,
        };

        eprintln!("{:?}", iter);

        while let Some((chunk_point, sample_iter)) = iter.next() {
            eprintln!("chunk_point: {:?}", chunk_point);
            eprintln!("sample_iter: {:?}", sample_iter);
        }
    }
}