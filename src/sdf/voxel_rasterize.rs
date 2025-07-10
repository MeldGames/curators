use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

use crate::sdf::{Sdf, ops};
use crate::voxel::{Voxel, Voxels};

pub struct RasterVoxel {
    pub point: IVec3,
    pub distance: f32,
}

#[derive(Debug, Clone)]
pub struct RasterConfig {
    pub clip_bounds: Aabb3d,
    pub grid_scale: Vec3,
}

pub struct RasterIterator<S: Sdf, I: Iterator<Item = IVec3>> {
    sdf: S,
    sample_points: I,
    config: RasterConfig,
}

impl<S: Sdf, I: Iterator<Item = IVec3>> Iterator for RasterIterator<S, I> {
    type Item = RasterVoxel;

    fn next(&mut self) -> Option<Self::Item> {
        let sample_point = self.sample_points.next()?;
        let distance = self.sdf.sdf(sample_point.as_vec3());
        Some(RasterVoxel { point: sample_point, distance })
    }
}

pub fn rasterize<S: Sdf>(
    sdf: S,
    config: RasterConfig,
) -> RasterIterator<ops::Scale<S>, impl Iterator<Item = IVec3>> {
    let aabb = sdf
        .aabb()
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
        sdf: ops::Scale { primitive: sdf, scale: 1.0 / config.grid_scale }, // might need to invert this scale?
        sample_points: point_iter,
        config,
    }
}
