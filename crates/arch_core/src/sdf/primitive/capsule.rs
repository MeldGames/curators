use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;
use crate::sdf::Sdf;

/// A capsule defined by two endpoints and a radius.
/// The capsule extends from `start` to `end` with the given `radius` and has spherical caps at both ends.
#[derive(Clone, Copy, Debug, Reflect)]
pub struct Capsule {
    /// Start point of the capsule
    pub start: Vec3,
    /// End point of the capsule
    pub end: Vec3,
    /// Radius of the capsule
    pub radius: f32,
}

impl Capsule {
    /// Create a new capsule
    pub fn new(start: Vec3, end: Vec3, radius: f32) -> Self {
        Self { start, end, radius }
    }
}

impl Sdf for Capsule {
    fn sdf(&self, point: Vec3) -> f32 {
        // Convert the GLSL algorithm to Rust
        let pa = point - self.start;
        let ba = self.end - self.start;
        
        // Calculate the parameter h that represents the closest point on the line segment
        let h = (pa.dot(ba) / ba.dot(ba)).clamp(0.0, 1.0);
        
        // Calculate the distance from the point to the closest point on the line segment, minus radius
        (pa - ba * h).length() - self.radius
    }
    
    fn aabb(&self) -> Option<Aabb3d> {
        // Calculate the bounding box that encompasses both endpoints and the radius
        let min_point = self.start.min(self.end) - Vec3::splat(self.radius);
        let max_point = self.start.max(self.end) + Vec3::splat(self.radius);
        
        Some(Aabb3d {
            min: min_point.into(),
            max: max_point.into(),
        })
    }
}
