use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

use serde::{Serialize, Deserialize};

use crate::sdf::Sdf;

/// A rounded box defined by size and rounding radius, centered at the origin.
#[derive(Clone, Copy, Debug, Reflect, Serialize, Deserialize)]
#[reflect(Default, Clone, Debug)]
pub struct RoundedBox {
    /// Size of the box (width, height, depth)
    pub size: Vec3,
    /// Rounding radius
    pub radius: f32,
}

impl RoundedBox {
    /// Create a new rounded box centered at the origin
    pub fn new(size: Vec3, radius: f32) -> Self {
        Self { size, radius }
    }
}

impl Default for RoundedBox {
    fn default() -> Self {
        Self { size: Vec3::ONE, radius: 0.0 }
    }
}

impl Sdf for RoundedBox {
    fn sdf(&self, point: Vec3) -> f32 {
        // GLSL: float sdRoundBox( vec3 p, vec3 b, float r ) { vec3 q = abs(p) - b;
        // return length(max(q,0.0)) + min(max(q.x,max(q.y,q.z)),0.0) - r; }
        let q = point.abs() - self.size * 0.5;
        q.max(Vec3::ZERO).length() + q.max_element().min(0.0) - self.radius
    }

    fn aabb(&self) -> Option<Aabb3d> {
        let half_size = self.size * 0.5 + Vec3::splat(self.radius);
        Some(Aabb3d { min: (-half_size).into(), max: (half_size).into() })
    }
}
