use crate::sdf::Sdf;
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

pub use bevy::math::primitives::Sphere;

impl Sdf for Sphere {
    fn sdf(&self, point: Vec3) -> f32 {
        point.length() - self.radius
    }
    fn aabb(&self) -> Option<Aabb3d> {
        Some(Aabb3d { min: Vec3A::splat(-self.radius), max: Vec3A::splat(self.radius) })
    }
}
