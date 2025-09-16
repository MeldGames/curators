use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

use crate::sdf::Sdf;

/// A triangle defined by three vertices.
#[derive(Clone, Copy, Debug, Reflect)]
#[reflect(Default, Clone, Debug)]
pub struct Triangle {
    /// First vertex
    pub v0: Vec3,
    /// Second vertex
    pub v1: Vec3,
    /// Third vertex
    pub v2: Vec3,
}

impl Triangle {
    /// Create a new triangle
    pub fn new(v0: Vec3, v1: Vec3, v2: Vec3) -> Self {
        Self { v0, v1, v2 }
    }
}

impl Default for Triangle {
    fn default() -> Self {
        Self {
            v0: Vec3::new(-0.5, 0.0, 0.0),
            v1: Vec3::new(0.5, 0.0, 0.0),
            v2: Vec3::new(0.0, 0.5, 0.0),
        }
    }
}

impl Sdf for Triangle {
    fn sdf(&self, point: Vec3) -> f32 {
        // GLSL: float sdTriangle( vec3 p, vec3 a, vec3 b, vec3 c )
        let v0v1 = self.v1 - self.v0;
        let v0v2 = self.v2 - self.v0;
        let v1v2 = self.v2 - self.v1;
        let v2v0 = self.v0 - self.v2;

        let nor = v0v1.cross(v0v2);

        let d = [v0v1.dot(v0v1), v0v2.dot(v0v2), v1v2.dot(v1v2), v2v0.dot(v2v0)];

        let q = [point - self.v0, point - self.v1, point - self.v2];

        let s = [v0v1.cross(q[0]), v1v2.cross(q[1]), v2v0.cross(q[2])];

        let c = [s[0].dot(nor), s[1].dot(nor), s[2].dot(nor)];

        if c[0] > 0.0 && c[1] > 0.0 && c[2] > 0.0 {
            (q[0].dot(nor) / nor.length()).abs()
        } else {
            let e = [
                v0v1 * (q[0].dot(v0v1) / d[0]).clamp(0.0, 1.0) - q[0],
                v1v2 * (q[1].dot(v1v2) / d[2]).clamp(0.0, 1.0) - q[1],
                v2v0 * (q[2].dot(v2v0) / d[3]).clamp(0.0, 1.0) - q[2],
            ];

            e.iter().map(|e| e.length()).fold(f32::INFINITY, f32::min)
        }
    }

    fn aabb(&self) -> Option<Aabb3d> {
        let min_point = self.v0.min(self.v1).min(self.v2);
        let max_point = self.v0.max(self.v1).max(self.v2);

        Some(Aabb3d { min: min_point.into(), max: max_point.into() })
    }
}
