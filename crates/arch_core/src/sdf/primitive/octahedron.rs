use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;
use serde::{Deserialize, Serialize};

use crate::sdf::{Sdf, SdfNode};

/// An octahedron defined by size, centered at the origin.
#[derive(Clone, Copy, Debug, Reflect, Serialize, Deserialize)]
#[reflect(Default, Clone, Debug)]
pub struct Octahedron {
    /// Size of the octahedron
    pub size: f32,
}

impl Octahedron {
    /// Create a new octahedron centered at the origin
    pub fn new(size: f32) -> Self {
        Self { size }
    }
}

impl Default for Octahedron {
    fn default() -> Self {
        Self { size: 1.0 }
    }
}

impl Sdf for Octahedron {
    fn sdf(&self, point: Vec3) -> f32 {
        // GLSL: float sdOctahedron( vec3 p, float s ) { p = abs(p); return
        // (p.x+p.y+p.z-s)*0.57735027; }
        let p = point.abs();
        (p.x + p.y + p.z - self.size) * 0.57735027
    }

    fn aabb(&self) -> Option<Aabb3d> {
        let half_size = self.size * 0.5;
        Some(Aabb3d { min: (-Vec3::splat(half_size)).into(), max: (Vec3::splat(half_size)).into() })
    }

    fn as_node(&self) -> SdfNode {
        SdfNode::Octahedron(*self)
    }
}
