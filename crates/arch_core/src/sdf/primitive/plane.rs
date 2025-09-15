use crate::sdf::Sdf;
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

/// A plane defined by a normal vector and distance from origin.
/// The plane equation is: normal · point + distance = 0
#[derive(Clone, Copy, Debug, Reflect)]
pub struct Plane {
    /// Normal vector of the plane (should be normalized)
    pub normal: Vec3,
    /// Distance from origin along the normal
    pub distance: f32,
}

impl Plane {
    /// Create a new plane
    pub fn new(normal: Vec3, distance: f32) -> Self {
        Self { normal, distance }
    }

    /// Create a plane from a point and normal
    pub fn from_point_normal(point: Vec3, normal: Vec3) -> Self {
        let distance = -point.dot(normal);
        Self { normal, distance }
    }
}

impl Sdf for Plane {
    fn sdf(&self, point: Vec3) -> f32 {
        // GLSL: float sdPlane( vec3 p, vec3 n, float h ) { return dot(p,n) + h; }
        point.dot(self.normal) + self.distance
    }

    fn aabb(&self) -> Option<Aabb3d> {
        // Planes are infinite, so no finite AABB
        None
    }
}
