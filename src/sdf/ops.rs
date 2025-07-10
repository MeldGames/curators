use crate::sdf::Sdf;
use bevy::prelude::*;
use bevy_math::bounding::{Aabb3d, BoundingVolume};

/// Scale the underlying primitive.
///
/// Non uniform scaling is supported, but may cause some issues.
pub struct Scale<P: Sdf> {
    pub primitive: P,
    pub scale: Vec3,
}

impl<P: Sdf> Sdf for Scale<P> {
    fn sdf(&self, point: Vec3) -> f32 {
        self.primitive.sdf(point / self.scale) * self.scale.x.min(self.scale.y.min(self.scale.z))
    }

    fn aabb(&self) -> Option<bevy_math::bounding::Aabb3d> {
        self.primitive.aabb().map(|aabb| aabb.scale_around_center(self.scale))
    }
}

pub struct Transform<P: Sdf> {
    pub rotation: Quat,
    pub translation: Vec3,
    pub primitive: P,
}

impl<P: Sdf> Sdf for Transform<P> {
    fn sdf(&self, point: Vec3) -> f32 {
        let inverted_point = self.rotation.inverse() * (point - self.translation);
        self.primitive.sdf(inverted_point)
    }

    fn aabb(&self) -> Option<bevy_math::bounding::Aabb3d> {
        self.primitive.aabb().map(|aabb| aabb.transformed_by(self.translation, self.rotation))
    }
}


pub struct Twist<P: Sdf> {
    pub strength: f32,
    pub primitive: P,
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

    fn aabb(&self) -> Option<bevy_math::bounding::Aabb3d> {
        self.primitive.aabb().map(|aabb| {
            Aabb3d {
                min: Vec3A::new(
                    aabb.min.x.min(aabb.min.z),
                    aabb.min.y,
                    aabb.min.x.min(aabb.min.z),
                ),
                max: Vec3A::new(
                    aabb.max.x.max(aabb.max.z),
                    aabb.max.y,
                    aabb.max.x.max(aabb.max.z),
                ),
            }
        })
    }
}