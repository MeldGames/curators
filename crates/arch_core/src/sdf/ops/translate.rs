use crate::sdf::Sdf;
use bevy::prelude::*;
use bevy_math::bounding::{Aabb3d, BoundingVolume};

/// Translate the underlying primitive.
#[derive(Debug, Clone)]
pub struct Translate<P: Sdf> {
    pub translation: Vec3,
    pub primitive: P,
}

impl<P: Sdf> Translate<P> {
    /// Create a new translation operation
    pub fn new(primitive: P, translation: Vec3) -> Self {
        Self { primitive, translation }
    }
}

impl<P: Sdf> Sdf for Translate<P> {
    fn sdf(&self, point: Vec3) -> f32 {
        self.primitive.sdf(point - self.translation)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        self.primitive.aabb().map(|aabb| aabb.translated_by(self.translation))
    }
}
