use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

use crate::sdf::Sdf;

/// A pyramid defined by height and base size, centered at the origin.
#[derive(Clone, Copy, Debug)]
pub struct Pyramid {
    /// Height of the pyramid
    pub height: f32,
    /// Size of the base (width, depth)
    pub base_size: Vec2,
}

impl Pyramid {
    /// Create a new pyramid centered at the origin
    pub fn new(height: f32, base_size: Vec2) -> Self {
        Self { height, base_size }
    }
}

impl Sdf for Pyramid {
    fn sdf(&self, point: Vec3) -> f32 {
        // GLSL: float sdPyramid( vec3 p, float h ) { float m2 = h*h + 0.25; p.xz =
        // abs(p.xz); p.xz = (p.z>p.x) ? p.zx : p.xz; p.xz -= 0.5; vec3 q = vec3( p.z,
        // h*p.y - 0.5*p.x, h*p.x + 0.5*p.y); float s = max(-q.x,0.0); float t = clamp(
        // (q.y-0.5*p.z)/(m2+0.25), 0.0, 1.0 ); float a = m2*(q.x+s)*(q.x+s) + q.y*q.y;
        // float b = m2*(q.x+0.5*t)*(q.x+0.5*t) + (q.y-m2*0.5*t)*(q.y-m2*0.5*t); float
        // d2 = min(q.y,-q.x*m2-q.y*0.5) > 0.0 ? 0.0 : min(a,b); return sqrt(
        // (d2+q.z*q.z)/m2 ) * sign(max(q.z,-p.y)); }
        let m2 = self.height * self.height + 0.25;
        let mut p = point;
        p.x = p.x.abs();
        p.z = p.z.abs();
        if p.z > p.x {
            let temp = p.x;
            p.x = p.z;
            p.z = temp;
        }
        p.x -= 0.5;
        let q = Vec3::new(p.z, self.height * p.y - 0.5 * p.x, self.height * p.x + 0.5 * p.y);
        let s = (-q.x).max(0.0);
        let t = ((q.y - 0.5 * p.z) / (m2 + 0.25)).clamp(0.0, 1.0);
        let a = m2 * (q.x + s) * (q.x + s) + q.y * q.y;
        let b =
            m2 * (q.x + 0.5 * t) * (q.x + 0.5 * t) + (q.y - m2 * 0.5 * t) * (q.y - m2 * 0.5 * t);
        let d2 = if q.y.min(-q.x * m2 - q.y * 0.5) > 0.0 { 0.0 } else { a.min(b) };
        ((d2 + q.z * q.z) / m2).sqrt() * (q.z.max(-p.y)).signum()
    }

    fn aabb(&self) -> Option<Aabb3d> {
        let half_base = self.base_size * 0.5;
        let min_point = Vec3::new(-half_base.x, 0.0, -half_base.y);
        let max_point = Vec3::new(half_base.x, self.height, half_base.y);

        Some(Aabb3d { min: min_point.into(), max: max_point.into() })
    }
}
