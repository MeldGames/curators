use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

use crate::sdf::Sdf;

impl Sdf for Cuboid {
    fn sdf(&self, point: Vec3) -> f32 {
        // GLSL: float sdBox( vec3 p, vec3 b ) { vec3 q = abs(p) - b; return
        // length(max(q,0.0)) + min(max(q.x,max(q.y,q.z)),0.0); }
        let q = point.abs() - self.half_size;
        q.max(Vec3::ZERO).length() + q.max_element().min(0.0)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        let half_size = self.half_size;
        Some(Aabb3d { min: (-half_size).into(), max: (half_size).into() })
    }
}
