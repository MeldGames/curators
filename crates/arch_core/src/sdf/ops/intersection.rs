use std::sync::Arc;

use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;
use serde::{Deserialize, Serialize};

use crate::sdf::{Sdf, SdfNode};

/// Intersection operation - combines two SDFs with a maximum operation.
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Default, Clone, Debug)]
#[reflect(where A: Clone + Default, B: Clone + Default)]
pub struct Intersection<A: Sdf, B: Sdf> {
    pub a: A,
    pub b: B,
}

impl<A: Sdf, B: Sdf> Intersection<A, B> {
    /// Create a new intersection operation
    pub fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<A: Sdf, B: Sdf> Sdf for Intersection<A, B> {
    fn sdf(&self, point: Vec3) -> f32 {
        let d1 = self.a.sdf(point);
        let d2 = self.b.sdf(point);
        d1.max(d2)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        match (self.a.aabb(), self.b.aabb()) {
            (Some(a), Some(b)) => {
                let min = a.min.max(b.min);
                let max = a.max.min(b.max);
                if min.x <= max.x && min.y <= max.y && min.z <= max.z {
                    Some(Aabb3d { min, max })
                } else {
                    None // No intersection
                }
            },
            _ => None,
        }
    }

    fn as_node(&self) -> SdfNode {
        SdfNode::Intersection(Intersection {
            a: Arc::new(self.a.as_node()),
            b: Arc::new(self.b.as_node()),
        })
    }
}

impl<A: Sdf + Default, B: Sdf + Default> Default for Intersection<A, B> {
    fn default() -> Self {
        Self { a: A::default(), b: B::default() }
    }
}
