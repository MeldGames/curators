use crate::sdf::Sdf;
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

/// An octahedron defined by a center point and size.
#[derive(Clone, Copy, Debug)]
pub struct Octahedron {
    /// Center point of the octahedron
    pub center: Vec3,
    /// Size of the octahedron
    pub size: f32,
}

impl Octahedron {
    /// Create a new octahedron
    pub fn new(center: Vec3, size: f32) -> Self {
        Self { center, size }
    }
}

impl Sdf for Octahedron {
    fn sdf(&self, point: Vec3) -> f32 {
        // GLSL: float sdOctahedron( vec3 p, float s ) { p = abs(p); return (p.x+p.y+p.z-s)*0.57735027; }
        let p = (point - self.center).abs();
        (p.x + p.y + p.z - self.size) * 0.57735027
    }

    fn aabb(&self) -> Option<Aabb3d> {
        let half_size = self.size * 0.5;
        Some(Aabb3d {
            min: (self.center - Vec3::splat(half_size)).into(),
            max: (self.center + Vec3::splat(half_size)).into(),
        })
    }
}
