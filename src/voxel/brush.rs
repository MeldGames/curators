
use bevy::prelude::*;
use bevy::math::primitives::*;

use crate::voxel::Voxels;

impl Voxels {
    pub fn apply_brush<B: Brush>(&mut self, center: IVec3, b: B, voxel: Voxel) {
        for relative_affected in b.local_affected_voxels() {
            self.set_voxel(relative_affected, voxel);
        }
    }
}

pub trait Brush {
    /// Voxels affected by this brush without any transformation.
    fn local_affected_voxels(&self) -> impl Iterator<Item = IVec3>;
}

#[derive(Debug, Clone)]
pub struct BakedBrush(Vec<IVec3>);

impl BakedBrush {
    pub fn new<B: Brush>(b: B) -> Self {
        Self(b.local_affected_voxels().collect())
    }
}

impl Brush for BakedBrush {
    fn local_affected_voxels(&self) -> impl Iterator<Item = IVec3> {
        self.0.iter().copied()
    }
}

impl Brush for Sphere {
    fn local_affected_voxels(&self) -> Vec<IVec3> {
        let mut list = Vec::new();
        let min = IVec3::splat((-self.radius).floor() as i32);
        let max = IVec3::splat(self.radius.ceil() as i32);

        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    let point = IVec3::new(x, y, z);
                    if self.contains_point(point.as_vec3()) {
                        list.push(point);
                    }
                }
            }
        }

        list
    }
}


impl Brush for Torus {
    fn local_affected_voxels(&self) -> Vec<IVec3> {
        let mut list = Vec::new();

        let total_radii = self.minor_radius + self.major_radius;
        let min = IVec3::splat((-total_radii).floor() as i32);
        let max = IVec3::splat(total_radii.ceil() as i32);

        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    let point = IVec3::new(x, y, z);
                    if self.contains_point(point.as_vec3()) {
                        list.push(pos);
                    }
                }
            }
        }

        list
    }
}


pub trait ContainsPoint {
    fn contains_point(&self, point: Vec3) -> bool;
}

impl ContainsPoint for Torus {
    fn contains_point(&self, point: Vec3) -> bool {
        let distance_to_axis = (point.x.powi(2) + point.y.powi(2)).sqrt();
        let difference = self.major_radius - distance_to_axis;
        let squared_distance_to_center_of_tube = difference.powi(2) + point.z.powi(2);

        squared_distance_to_center_of_tube < self.minor_radius.powi(2)
    }
}

impl ContainsPoint for Sphere {
    fn contains_point(&self, point: Vec3) -> bool {
        point.length() <= self.radius
    }
}