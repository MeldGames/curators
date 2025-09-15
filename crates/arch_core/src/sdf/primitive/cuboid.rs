use crate::sdf::Sdf;
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

/// A cuboid (AABB) defined by a center point and size.
/// The cuboid extends from center - size/2 to center + size/2.
#[derive(Clone, Copy, Debug)]
pub struct Cuboid {
    /// Center point of the cuboid
    pub center: Vec3,
    /// Size of the cuboid (width, height, depth)
    pub size: Vec3,
}

impl Cuboid {
    /// Create a new cuboid
    pub fn new(center: Vec3, size: Vec3) -> Self {
        Self { center, size }
    }
}

impl Sdf for Cuboid {
    fn sdf(&self, point: Vec3) -> f32 {
        // GLSL: float sdBox( vec3 p, vec3 b ) { vec3 q = abs(p) - b; return length(max(q,0.0)) + min(max(q.x,max(q.y,q.z)),0.0); }
        let q = (point - self.center).abs() - self.size * 0.5;
        q.max(Vec3::ZERO).length() + q.max_element().min(0.0)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        let half_size = self.size * 0.5;
        Some(Aabb3d {
            min: (self.center - half_size).into(),
            max: (self.center + half_size).into(),
        })
    }
}
