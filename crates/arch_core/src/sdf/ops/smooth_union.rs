use crate::sdf::Sdf;
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

// Helper functions
fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.max(min).min(max)
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

/// Smooth Union operation - combines two SDFs with smooth blending.
#[derive(Debug, Clone)]
pub struct SmoothUnion<A: Sdf, B: Sdf> {
    pub a: A,
    pub b: B,
    pub k: f32,
}

impl<A: Sdf, B: Sdf> SmoothUnion<A, B> {
    /// Create a new smooth union operation
    pub fn new(a: A, b: B, k: f32) -> Self {
        Self { a, b, k }
    }
}

impl<A: Sdf, B: Sdf> Sdf for SmoothUnion<A, B> {
    fn sdf(&self, point: Vec3) -> f32 {
        let d1 = self.a.sdf(point);
        let d2 = self.b.sdf(point);
        let h = clamp(0.5 + 0.5 * (d2 - d1) / self.k, 0.0, 1.0);
        mix(d2, d1, h) - self.k * h * (1.0 - h)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        match (self.a.aabb(), self.b.aabb()) {
            (Some(a), Some(b)) => {
                // Smooth union can extend beyond both shapes due to blending
                let expansion = Vec3A::splat(self.k);
                Some(Aabb3d {
                    min: a.min.min(b.min) - expansion,
                    max: a.max.max(b.max) + expansion,
                })
            },
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        }
    }
}
