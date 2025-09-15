use crate::sdf::Sdf;
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

/// A cone defined by two endpoints and their respective radii.
/// The cone extends from point a (radius_a) to point b (radius_b).
#[derive(Clone, Copy, Debug, Reflect)]
pub struct Cone {
    /// Start point of the cone
    pub a: Vec3,
    /// End point of the cone
    pub b: Vec3,
    /// Radius at point a
    pub radius_a: f32,
    /// Radius at point b
    pub radius_b: f32,
}

impl Cone {
    /// Create a new cone
    pub fn new(a: Vec3, b: Vec3, radius_a: f32, radius_b: f32) -> Self {
        Self { a, b, radius_a, radius_b }
    }
}

impl Sdf for Cone {
    fn sdf(&self, point: Vec3) -> f32 {
        // GLSL: float sdCone( vec3 p, vec3 a, vec3 b, float ra, float rb )
        let ba = self.b - self.a;
        let pa = point - self.a;
        let baba = ba.dot(ba);
        let paba = pa.dot(ba);
        let x = (pa * baba - ba * paba).length() - self.radius_a * baba;
        let y = (paba - baba * 0.5).abs() - baba * 0.5;
        let x2 = x * x;
        let y2 = y * y * baba;
        let d = if x.max(y) < 0.0 {
            -x2.min(y2)
        } else {
            (if x > 0.0 { x2 } else { 0.0 }) + (if y > 0.0 { y2 } else { 0.0 })
        };
        d.signum() * d.abs().sqrt() / baba
    }

    fn aabb(&self) -> Option<Aabb3d> {
        // Calculate the bounding box that encompasses both endpoints and their radii
        let max_radius = self.radius_a.max(self.radius_b);
        let min_point = self.a.min(self.b) - Vec3::splat(max_radius);
        let max_point = self.a.max(self.b) + Vec3::splat(max_radius);

        Some(Aabb3d { min: min_point.into(), max: max_point.into() })
    }
}
