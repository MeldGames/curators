use crate::sdf::Sdf;
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

/// Rotate the underlying primitive.
#[derive(Debug, Clone)]
pub struct Rotation<P: Sdf> {
    pub rotation: Quat,
    pub primitive: P,
}

impl<P: Sdf> Rotation<P> {
    /// Create a new rotation operation
    pub fn new(primitive: P, rotation: Quat) -> Self {
        Self { primitive, rotation }
    }
}

impl<P: Sdf> Sdf for Rotation<P> {
    fn sdf(&self, point: Vec3) -> f32 {
        let inverted_point = self.rotation.inverse() * point;
        self.primitive.sdf(inverted_point)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        self.primitive.aabb().map(|aabb| aabb.rotated_by(self.rotation))
    }
}
