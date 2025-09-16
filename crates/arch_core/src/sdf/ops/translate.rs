use bevy::prelude::*;
use bevy_math::bounding::{Aabb3d, BoundingVolume};

use crate::sdf::Sdf;

/// Translate the underlying primitive.
#[derive(Debug, Clone, Reflect)]
pub struct Translate<P: Sdf> {
    pub translate: Vec3,
    pub primitive: P,
}

impl<P: Sdf> Translate<P> {
    /// Create a new translation operation
    pub fn new(primitive: P, translate: Vec3) -> Self {
        Self { primitive, translate }
    }
}

impl<P: Sdf> Sdf for Translate<P> {
    fn sdf(&self, point: Vec3) -> f32 {
        self.primitive.sdf(point - self.translate)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        self.primitive.aabb().map(|aabb| aabb.translated_by(self.translate))
    }
}
