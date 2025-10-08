pub use bevy::math::primitives::Sphere;
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

use crate::sdf::{Sdf, SdfNode};

impl Sdf for Sphere {
    fn sdf(&self, point: Vec3) -> f32 {
        point.length() - self.radius
    }

    fn aabb(&self) -> Option<Aabb3d> {
        Some(Aabb3d { min: Vec3A::splat(-self.radius), max: Vec3A::splat(self.radius) })
    }

    fn as_node(&self) -> SdfNode {
        SdfNode::Sphere(*self)
    }
}
