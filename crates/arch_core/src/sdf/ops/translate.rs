
use bevy::prelude::*;
use bevy_math::bounding::{Aabb3d, BoundingVolume};
use serde::{Serialize, Deserialize};

use crate::sdf::Sdf;

/// Translate the underlying primitive.
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Default, Clone, Debug)]
#[reflect(where P: Clone + Default + Reflect)]
// #[serde(bound(serialize = "P: Serialize", deserialize = "P: for<'de> Deserialize<'de>"))]
pub struct Translate<P: Sdf> {
    pub translate: Vec3,
    pub primitive: P,
}

impl<S: Sdf + Default> Default for Translate<S> {
    fn default() -> Self {
        Self { translate: Vec3::ZERO, primitive: default() }
    }
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
