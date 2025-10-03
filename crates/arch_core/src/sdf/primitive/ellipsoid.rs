use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;
use serde::{Serialize, Deserialize};

use crate::sdf::Sdf;

/// An ellipsoid defined by radii along each axis, centered at the origin.
#[derive(Clone, Copy, Debug, Reflect, Serialize, Deserialize)]
#[reflect(Default, Clone, Debug)]
pub struct Ellipsoid {
    /// Radii along x, y, z axes
    pub radii: Vec3,
}

impl Ellipsoid {
    /// Create a new ellipsoid centered at the origin
    pub fn new(radii: Vec3) -> Self {
        Self { radii }
    }
}

impl Default for Ellipsoid {
    fn default() -> Self {
        Self { radii: Vec3::ONE }
    }
}

impl Sdf for Ellipsoid {
    fn sdf(&self, point: Vec3) -> f32 {
        // GLSL: float sdEllipsoid( vec3 p, vec3 r ) { float k0 = length(p/r); float k1
        // = length(p/(r*r)); return k0*(k0-1.0)/k1; }
        let p = point;
        let k0 = (p / self.radii).length();
        let k1 = (p / (self.radii * self.radii)).length();
        k0 * (k0 - 1.0) / k1
    }

    fn aabb(&self) -> Option<Aabb3d> {
        Some(Aabb3d { min: (-self.radii).into(), max: (self.radii).into() })
    }
}
