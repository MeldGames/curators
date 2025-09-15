use crate::sdf::Sdf;
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

/// A rounded box defined by a center point, size, and rounding radius.
#[derive(Clone, Copy, Debug)]
pub struct RoundedBox {
    /// Center point of the box
    pub center: Vec3,
    /// Size of the box (width, height, depth)
    pub size: Vec3,
    /// Rounding radius
    pub radius: f32,
}

impl RoundedBox {
    /// Create a new rounded box
    pub fn new(center: Vec3, size: Vec3, radius: f32) -> Self {
        Self { center, size, radius }
    }
}

impl Sdf for RoundedBox {
    fn sdf(&self, point: Vec3) -> f32 {
        // GLSL: float sdRoundBox( vec3 p, vec3 b, float r ) { vec3 q = abs(p) - b; return length(max(q,0.0)) + min(max(q.x,max(q.y,q.z)),0.0) - r; }
        let q = (point - self.center).abs() - self.size * 0.5;
        q.max(Vec3::ZERO).length() + q.max_element().min(0.0) - self.radius
    }

    fn aabb(&self) -> Option<Aabb3d> {
        let half_size = self.size * 0.5 + Vec3::splat(self.radius);
        Some(Aabb3d {
            min: (self.center - half_size).into(),
            max: (self.center + half_size).into(),
        })
    }
}
