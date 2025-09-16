use bevy::prelude::*;
use bevy_math::bounding::{Aabb3d, BoundingVolume};

use crate::sdf::Sdf;

/// Rotate the underlying primitive.
#[derive(Debug, Clone, Reflect)]
pub struct Rotate<P: Sdf> {
    pub rotate: Quat,
    pub primitive: P,
}

impl<P: Sdf> Rotate<P> {
    /// Create a new rotation operation
    pub fn new(primitive: P, rotate: Quat) -> Self {
        Self { primitive, rotate }
    }
}

impl<P: Sdf> Sdf for Rotate<P> {
    fn sdf(&self, point: Vec3) -> f32 {
        let inverted_point = self.rotate.inverse() * point;
        self.primitive.sdf(inverted_point)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        self.primitive.aabb().map(|aabb| aabb.rotated_by(self.rotate))
    }
}
