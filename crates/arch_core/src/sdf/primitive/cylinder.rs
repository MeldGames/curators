use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;
use serde::{Serialize, Deserialize};

use crate::sdf::Sdf;

/// A capped cylinder defined by two endpoints and a radius.
/// The cylinder extends from `start` to `end` with the given `radius`.
#[derive(Clone, Copy, Debug, Reflect, Serialize, Deserialize)]
#[reflect(Default, Clone, Debug)]
pub struct Cylinder {
    /// Start point of the cylinder
    pub start: Vec3,
    /// End point of the cylinder
    pub end: Vec3,
    /// Radius of the cylinder
    pub radius: f32,
}

impl Default for Cylinder {
    fn default() -> Self {
        Self::new(Vec3::new(0.0, -0.5, 0.0), Vec3::new(0.0, 0.5, 0.0), 0.5)
    }
}

impl Cylinder {
    /// Create a new capped cylinder
    pub fn new(start: Vec3, end: Vec3, radius: f32) -> Self {
        Self { start, end, radius }
    }
}

impl Sdf for Cylinder {
    fn sdf(&self, point: Vec3) -> f32 {
        // Convert the GLSL algorithm to Rust
        let ba = self.end - self.start;
        let pa = point - self.start;
        let baba = ba.dot(ba);
        let paba = pa.dot(ba);

        // Calculate the distance from the cylinder axis
        let x = (pa * baba - ba * paba).length() - self.radius * baba;

        // Calculate the distance from the cylinder caps
        let y = (paba - baba * 0.5).abs() - baba * 0.5;

        let x2 = x * x;
        let y2 = y * y * baba;

        // Combine the distances using the same logic as the GLSL version
        let d = if x.max(y) < 0.0 {
            -x2.min(y2)
        } else {
            (if x > 0.0 { x2 } else { 0.0 }) + (if y > 0.0 { y2 } else { 0.0 })
        };

        d.signum() * d.abs().sqrt() / baba
    }

    fn aabb(&self) -> Option<Aabb3d> {
        // Calculate the bounding box that encompasses both endpoints and the radius
        let min_point = self.start.min(self.end) - Vec3::splat(self.radius);
        let max_point = self.start.max(self.end) + Vec3::splat(self.radius);

        Some(Aabb3d { min: min_point.into(), max: max_point.into() })
    }
}
