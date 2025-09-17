use bevy::prelude::*;
use bevy_math::bounding::{Aabb3d, BoundingVolume};

use crate::sdf::Sdf;

/// Scale the underlying primitive.
///
/// Non uniform scaling is supported, but may cause some issues.
#[derive(Debug, Clone, Reflect)]
#[reflect(Default, Clone, Debug)]
#[reflect(where S: Clone + Default)]
pub struct Scale<S: Sdf> {
    pub primitive: S,
    pub scale: Vec3,
}

impl<P: Sdf> Scale<P> {
    /// Create a new scale operation
    pub fn new(primitive: P, scale: Vec3) -> Self {
        Self { primitive, scale }
    }
}

impl<S: Sdf + Default> Default for Scale<S> {
    fn default() -> Self {
        Self { primitive: default(), scale: Vec3::ONE }
    }
}

impl<P: Sdf> Sdf for Scale<P> {
    fn sdf(&self, point: Vec3) -> f32 {
        self.primitive.sdf(point / self.scale) * self.scale.x.min(self.scale.y.min(self.scale.z))
    }

    fn aabb(&self) -> Option<Aabb3d> {
        self.primitive.aabb().map(|aabb| aabb.scale_around_center(self.scale))
    }
}
