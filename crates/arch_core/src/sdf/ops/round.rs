use std::sync::Arc;

use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

use crate::sdf::{Sdf, SdfNode};

/// Round operation - adds rounding to the underlying primitive.
#[derive(Debug, Clone, Reflect)]
#[reflect(Default, Clone, Debug)]
pub struct Round<P: Sdf> {
    pub primitive: P,
    pub radius: f32,
}

impl<P: Sdf> Round<P> {
    /// Create a new round operation
    pub fn new(primitive: P, radius: f32) -> Self {
        Self { primitive, radius }
    }
}

impl<S: Sdf> Default for Round<S> {
    fn default() -> Self {
        Self { primitive: S::default(), radius: 0.0 }
    }
}

impl<P: Sdf> Sdf for Round<P> {
    fn sdf(&self, point: Vec3) -> f32 {
        self.primitive.sdf(point) - self.radius
    }

    fn aabb(&self) -> Option<Aabb3d> {
        self.primitive.aabb().map(|aabb| {
            let expansion = Vec3A::splat(self.radius);
            Aabb3d { min: aabb.min - expansion, max: aabb.max + expansion }
        })
    }
}
