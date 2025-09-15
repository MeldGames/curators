use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

use crate::sdf::Sdf;

/// A cuboid (AABB) defined by size, centered at the origin.
/// The cuboid extends from -size/2 to +size/2.
#[derive(Clone, Copy, Debug, Reflect)]
pub struct Cuboid {
    /// Size of the cuboid (width, height, depth)
    pub size: Vec3,
}

impl Cuboid {
    /// Create a new cuboid centered at the origin
    pub fn new(size: Vec3) -> Self {
        Self { size }
    }
}

impl Sdf for Cuboid {
    fn sdf(&self, point: Vec3) -> f32 {
        // GLSL: float sdBox( vec3 p, vec3 b ) { vec3 q = abs(p) - b; return
        // length(max(q,0.0)) + min(max(q.x,max(q.y,q.z)),0.0); }
        let q = point.abs() - self.size * 0.5;
        q.max(Vec3::ZERO).length() + q.max_element().min(0.0)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        let half_size = self.size * 0.5;
        Some(Aabb3d { min: (-half_size).into(), max: (half_size).into() })
    }
}
