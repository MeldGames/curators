use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

use crate::sdf::Sdf;

// Helper functions
fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.max(min).min(max)
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

/// Smooth Subtraction operation - smoothly subtracts the second SDF from the
/// first.
#[derive(Debug, Clone, Reflect)]
#[reflect(Default, Clone, Debug)]
pub struct SmoothSubtraction<A: Sdf, B: Sdf> {
    pub a: A,
    pub b: B,
    pub k: f32,
}

impl<A: Sdf, B: Sdf> SmoothSubtraction<A, B> {
    /// Create a new smooth subtraction operation
    pub fn new(a: A, b: B, k: f32) -> Self {
        Self { a, b, k }
    }
}

impl<A: Sdf, B: Sdf> Sdf for SmoothSubtraction<A, B> {
    fn sdf(&self, point: Vec3) -> f32 {
        let d1 = self.a.sdf(point);
        let d2 = self.b.sdf(point);
        let h = clamp(0.5 - 0.5 * (d2 + d1) / self.k, 0.0, 1.0);
        mix(d2, -d1, h) + self.k * h * (1.0 - h)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        // Smooth subtraction can extend the result slightly
        self.b.aabb().map(|aabb| {
            let expansion = Vec3A::splat(self.k);
            Aabb3d { min: aabb.min - expansion, max: aabb.max + expansion }
        })
    }
}

impl<A: Sdf, B: Sdf> Default for SmoothSubtraction<A, B> {
    fn default() -> Self {
        Self { a: A::default(), b: B::default(), k: 0.0 }
    }
}
