use crate::sdf::Sdf;
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

/// A hexagonal prism defined by a center point and size.
#[derive(Clone, Copy, Debug)]
pub struct HexagonalPrism {
    /// Center point of the prism
    pub center: Vec3,
    /// Size of the prism (width, height, depth)
    pub size: Vec3,
}

impl HexagonalPrism {
    /// Create a new hexagonal prism
    pub fn new(center: Vec3, size: Vec3) -> Self {
        Self { center, size }
    }
}

impl Sdf for HexagonalPrism {
    fn sdf(&self, point: Vec3) -> f32 {
        // GLSL: float sdHexPrism( vec3 p, vec2 h ) { vec3 q = abs(p); return max(q.z-h.y,max((q.x*0.866025+q.y*0.5),q.y)-h.x); }
        let p = (point - self.center).abs();
        let h = Vec2::new(self.size.x, self.size.z) * 0.5;
        p.z.max(h.y).max((p.x * 0.866025 + p.y * 0.5).max(p.y) - h.x)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        let half_size = self.size * 0.5;
        Some(Aabb3d {
            min: (self.center - half_size).into(),
            max: (self.center + half_size).into(),
        })
    }
}
