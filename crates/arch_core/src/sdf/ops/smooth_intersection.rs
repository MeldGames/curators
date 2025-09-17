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

/// Smooth Intersection operation - smoothly intersects two SDFs.
#[derive(Debug, Clone, Reflect)]
#[reflect(Default, Clone, Debug)]
#[reflect(where A: Clone + Default, B: Clone + Default)]
pub struct SmoothIntersection<A: Sdf, B: Sdf> {
    pub a: A,
    pub b: B,
    pub k: f32,
}

impl<A: Sdf, B: Sdf> SmoothIntersection<A, B> {
    /// Create a new smooth intersection operation
    pub fn new(a: A, b: B, k: f32) -> Self {
        Self { a, b, k }
    }
}

impl<A: Sdf, B: Sdf> Sdf for SmoothIntersection<A, B> {
    fn sdf(&self, point: Vec3) -> f32 {
        let d1 = self.a.sdf(point);
        let d2 = self.b.sdf(point);
        let h = clamp(0.5 - 0.5 * (d2 - d1) / self.k, 0.0, 1.0);
        mix(d2, d1, h) + self.k * h * (1.0 - h)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        match (self.a.aabb(), self.b.aabb()) {
            (Some(a), Some(b)) => {
                // Smooth intersection can extend slightly beyond the intersection
                let min = a.min.max(b.min);
                let max = a.max.min(b.max);
                if min.x <= max.x && min.y <= max.y && min.z <= max.z {
                    let expansion = Vec3A::splat(self.k);
                    Some(Aabb3d { min: min - expansion, max: max + expansion })
                } else {
                    None // No intersection
                }
            },
            _ => None,
        }
    }
}

impl<A: Sdf + Default, B: Sdf + Default> Default for SmoothIntersection<A, B> {
    fn default() -> Self {
        Self { a: A::default(), b: B::default(), k: 0.0 }
    }
}
