use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;
use serde::{Serialize, Deserialize};

use crate::sdf::Sdf;

/// Subtraction operation - subtracts the second SDF from the first.
#[derive(Debug, Clone, Default, Reflect, Serialize, Deserialize)]
#[reflect(Default, Clone, Debug)]
#[reflect(where A: Clone + Default, B: Clone + Default)]
pub struct Subtraction<A: Sdf, B: Sdf> {
    pub a: A,
    pub b: B,
}

impl<A: Sdf, B: Sdf> Subtraction<A, B> {
    /// Create a new subtraction operation
    pub fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<A: Sdf, B: Sdf> Sdf for Subtraction<A, B> {
    fn sdf(&self, point: Vec3) -> f32 {
        let d1 = self.a.sdf(point);
        let d2 = self.b.sdf(point);
        (-d1).max(d2)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        // Subtraction can only shrink the result, so use the second operand's bounds
        self.b.aabb()
    }
}
