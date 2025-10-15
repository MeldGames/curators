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
pub struct RasterIter<S: Sdf, I: Iterator<Item = IVec3>> {
    sdf: S,
    sample_points: I,
}

impl<S: Sdf, I: Iterator<Item = IVec3>> Iterator for RasterIter<S, I> {
    type Item = RasterVoxel;

    fn next(&mut self) -> Option<Self::Item> {
        let sample_point = self.sample_points.next()?;
        let distance = self.sdf.sdf(sample_point.as_vec3());
        Some(RasterVoxel { point: sample_point, distance })
    }
}

#[derive(Debug)]
pub struct ChunkIntersectIter {
    world_min: IVec3,
    world_max: IVec3,

    chunk_points: PointIter,
    chunk_width: i32,
}

impl Iterator for ChunkIntersectIter {
    type Item = (ChunkPoint, PointIter);

    fn next(&mut self) -> Option<Self::Item> {
        let chunk_point = self.chunk_points.next()?;
        let local_point_iter = PointIter::clamped_to_chunk(self.world_min, self.world_max, chunk_point, self.chunk_width);
        Some((ChunkPoint(chunk_point), local_point_iter))
    }
}

impl ChunkIntersectIter {
    pub fn new(world_min: IVec3, world_max: IVec3, chunk_width: i32) -> Self {
        let chunk_min = world_min / chunk_width;
        let chunk_max = world_max / chunk_width;

        Self {
            world_min: world_min,
            world_max: world_max,
            chunk_points: PointIter::new(chunk_min, chunk_max),
            chunk_width,
        }
    }

    pub fn from_sdf<S: Sdf + Clone>(sdf: S, chunk_width: i32) -> Self {
        info!("from sdf: {:?}", &sdf);
        let aabb = sdf
            .aabb()
            .expect("Sampling an sdf needs to have proper bounds, if using an unbounded Sdf, use `Bounded::new(sdf, min, max)`");

        let min = aabb.min.floor().as_ivec3();
        let max = aabb.max.ceil().as_ivec3();
        info!("min: {:?}, max: {:?}", min, max);
        Self::new(min, max, chunk_width)
    }
}

// Clamp local sample points of a chunk
#[derive(Debug, Clone)]
pub struct PointIter {
    min: IVec3,

    current: i32,
    length: i32,
    bounds: IVec3,
}

impl PointIter {
    pub fn new(min: IVec3, max: IVec3) -> Self {
        let bounds = max - min + IVec3::ONE;
        let length = bounds.x * bounds.y * bounds.z;
        Self {
            min: min,

            current: 0,
            bounds: bounds,
            length: length,
        }
    }

    pub fn clamped_to_chunk(world_min: IVec3, world_max: IVec3, chunk_point: IVec3, chunk_width: i32) -> Self {
        // chunk point in voxel world space
        let min_chunk_point = chunk_point * chunk_width;
        // let max_chunk_point = min_chunk_point + IVec3::splat(chunk_width);

        // get the relative world space points
        let relative_min = world_min - min_chunk_point;
        let relative_max = world_max - min_chunk_point;

        // println!("relative: {:?}..{:?}", relative_min, relative_max);

        let min = relative_min.clamp(IVec3::ZERO, IVec3::splat(chunk_width - 1));
        let max = relative_max.clamp(IVec3::ZERO, IVec3::splat(chunk_width - 1));

        // Self::new(min_chunk_point + min, min_chunk_point + max)
        Self::new( min,  max)
    }

    pub fn from_sdf(sdf: impl Sdf) -> Self {
        let aabb = sdf
            .aabb()
            .expect("Sampling an sdf needs to have proper bounds, if using an unbounded Sdf, use `Bounded::new(sdf, min, max)`");

        let min = aabb.min.floor().as_ivec3();
        let max = aabb.max.ceil().as_ivec3();
        Self::new(min, max)
    }
}

impl Iterator for PointIter {
    type Item = IVec3;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.length {
            return None;
        } 

        // delinearize point
        let y = self.current / (self.bounds.x * self.bounds.z);
        let x = (self.current / self.bounds.z) % self.bounds.x;
        let z = self.current % self.bounds.z;

        let next_point = self.min + ivec3(x as i32, y as i32, z as i32);
        self.current += 1;
        Some(next_point)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.length - self.current) as usize;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for PointIter { }

/*
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

    let point_iter = PointIter::new(min, max);

    RasterIterator {
        sdf: ops::Scale { primitive: sdf, scale: 1.0 / config.grid_scale }, /* might need to invert this scale? */
        sample_points: point_iter,
        // config,
    }
}

pub fn rasterize_chunkwise<S: Sdf>(
    origin: IVec3,
    sdf: S,
    config: RasterConfig,
    chunk_width: i32, 
) -> RasterChunkIterator<ops::Scale<S>, impl Iterator<Item = IVec3>> {
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

    ChunkIntersectIter::new(min, max, 16)

    // RasterChunkIterator {
    //     sdf: ops::Scale { primitive: sdf, scale: 1.0 / config.grid_scale }, /* might need to invert this scale? */
    //     min: min,
    //     max: max,

    //     chunk_points: PointIter::new(min_chunk, max_chunk),
    //     chunk_width: 16,
    // }
}
*/


#[cfg(test)]
mod test {
    use crate::sdf::{self, voxel_rasterize::{PointIter, ChunkIntersectIter}, Sdf};
    use bevy::prelude::*;

    #[test]
    fn point_iter_clamped_to_chunk() {
        let chunk_width = 16;

        // basic origin samples, no clipping
        let min = IVec3::splat(0);
        let max = IVec3::splat(16);
        let mut samples_iter = PointIter::clamped_to_chunk(min, max, IVec3::new(0, 0, 0), chunk_width);

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
        let mut samples_iter = PointIter::clamped_to_chunk(min, max, IVec3::new(0, 0, 0), chunk_width);

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
        let mut samples_iter = PointIter::clamped_to_chunk(min, max, IVec3::new(1, 0, 0), chunk_width);
        assert_eq!(samples_iter.next(), None);

        // oob max, in bound min
        let min = IVec3::splat(2);
        let max = IVec3::splat(30);
        let mut samples_iter = PointIter::clamped_to_chunk(min, max, IVec3::new(0, 0, 0), chunk_width);

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
        let mut samples_iter = PointIter::clamped_to_chunk(min, max, IVec3::new(1, 1, 1), chunk_width);

        // 30 - 16 = 14
        let valid_points = (0..14).flat_map(move |y| {
            (0..14).flat_map(move |x| (0..14).map(move |z| IVec3::new(x, y, z) + IVec3::splat(chunk_width)))
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

        let chunk_points = PointIter::new(chunk_min, chunk_max);
        let mut iter = ChunkIntersectIter::from_sdf(sdf, 16);

        eprintln!("{:?}", iter);

        while let Some((chunk_point, sample_iter)) = iter.next() {
            eprintln!("chunk_point: {:?}", chunk_point);
            eprintln!("sample_iter: {:?}", sample_iter);
        }
    }
}