use crate::sdf::Sdf;
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

/// An ellipsoid defined by a center point and radii along each axis.
#[derive(Clone, Copy, Debug)]
pub struct Ellipsoid {
    /// Center point of the ellipsoid
    pub center: Vec3,
    /// Radii along x, y, z axes
    pub radii: Vec3,
}

impl Ellipsoid {
    /// Create a new ellipsoid
    pub fn new(center: Vec3, radii: Vec3) -> Self {
        Self { center, radii }
    }
}

impl Sdf for Ellipsoid {
    fn sdf(&self, point: Vec3) -> f32 {
        // GLSL: float sdEllipsoid( vec3 p, vec3 r ) { float k0 = length(p/r); float k1 = length(p/(r*r)); return k0*(k0-1.0)/k1; }
        let p = point - self.center;
        let k0 = (p / self.radii).length();
        let k1 = (p / (self.radii * self.radii)).length();
        k0 * (k0 - 1.0) / k1
    }

    fn aabb(&self) -> Option<Aabb3d> {
        Some(Aabb3d {
            min: (self.center - self.radii).into(),
            max: (self.center + self.radii).into(),
        })
    }
}
