use crate::sdf::Sdf;
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

/// Twist the underlying primitive around the Y axis.
#[derive(Debug, Clone)]
pub struct Twist<P: Sdf> {
    pub strength: f32,
    pub primitive: P,
}

impl<P: Sdf> Twist<P> {
    /// Create a new twist operation
    pub fn new(primitive: P, strength: f32) -> Self {
        Self { primitive, strength }
    }
}

impl<P: Sdf> Sdf for Twist<P> {
    fn sdf(&self, point: Vec3) -> f32 {
        let c = (self.strength * point.y).cos();
        let s = (self.strength * point.y).sin();

        let m = mat2(vec2(c, -s), vec2(s, c));

        let rotated_xz = m.mul_vec2(vec2(point.x, point.z));
        let rotated_point = Vec3::new(rotated_xz.x, point.y, rotated_xz.y);

        self.primitive.sdf(rotated_point)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        self.primitive.aabb().map(|aabb| Aabb3d {
            min: Vec3A::new(aabb.min.x.min(aabb.min.z), aabb.min.y, aabb.min.x.min(aabb.min.z)),
            max: Vec3A::new(aabb.max.x.max(aabb.max.z), aabb.max.y, aabb.max.x.max(aabb.max.z)),
        })
    }
}
