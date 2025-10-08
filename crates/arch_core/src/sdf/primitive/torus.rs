pub use bevy::math::primitives::Torus;
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

use crate::sdf::{Sdf, SdfNode};

impl Sdf for Torus {
    fn sdf(&self, point: Vec3) -> f32 {
        // Convert to cylindrical coordinates (distance from y-axis in xz plane)
        let xz_distance = (point.x * point.x + point.z * point.z).sqrt();

        // Distance from the torus center ring to the point
        let ring_distance = xz_distance - self.major_radius;

        // Distance in the tube cross-section (ring_distance, y)
        let tube_distance = (ring_distance * ring_distance + point.y * point.y).sqrt();

        tube_distance - self.minor_radius
    }

    fn aabb(&self) -> Option<Aabb3d> {
        let total_radii = self.minor_radius + self.major_radius;
        // xz using both radii and y using only the minor radius (the radius of the
        // tube).
        Some(Aabb3d {
            min: Vec3A::new(-total_radii, -self.minor_radius, -total_radii),
            max: Vec3A::new(total_radii, self.minor_radius, total_radii),
        })
    }

    fn as_node(&self) -> SdfNode {
        SdfNode::Torus(*self)
    }
}
