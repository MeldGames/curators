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
