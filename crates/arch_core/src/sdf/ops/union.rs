use std::sync::Arc;

use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;
use serde::{Deserialize, Serialize};

use crate::sdf::{Sdf, SdfNode};

/// Union operation - combines two SDFs with a minimum operation.
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Default, Clone, Debug)]
#[reflect(where A: Clone + Default, B: Clone + Default)]
pub struct Union<A: Sdf, B: Sdf> {
    pub a: A,
    pub b: B,
}

impl<A: Sdf, B: Sdf> Union<A, B> {
    /// Create a new union operation
    pub fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<A: Sdf + Default, B: Sdf + Default> Default for Union<A, B> {
    fn default() -> Self {
        Self { a: A::default(), b: B::default() }
    }
}

impl<A: Sdf, B: Sdf> Sdf for Union<A, B> {
    fn sdf(&self, point: Vec3) -> f32 {
        let d1 = self.a.sdf(point);
        let d2 = self.b.sdf(point);
        d1.min(d2)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        match (self.a.aabb(), self.b.aabb()) {
            (Some(a), Some(b)) => Some(Aabb3d { min: a.min.min(b.min), max: a.max.max(b.max) }),
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        }
    }

    fn as_node(&self) -> SdfNode {
        SdfNode::Union(Union { a: Arc::new(self.a.as_node()), b: Arc::new(self.b.as_node()) })
    }
}
