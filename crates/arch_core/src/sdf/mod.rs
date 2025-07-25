//! SDF (Signed distance functions) for voxel rasterization.

use std::f32::consts::PI;

pub use bevy::math::primitives::{Sphere, Torus};
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

pub mod ops;
pub mod voxel_rasterize;

pub trait Sdf {
    fn sdf(&self, point: Vec3) -> f32;
    fn aabb(&self) -> Option<Aabb3d>;
}

impl<S: Sdf> Sdf for Box<S> {
    fn sdf(&self, point: Vec3) -> f32 {
        S::sdf(&*self, point)
    }
    fn aabb(&self) -> Option<Aabb3d> {
        S::aabb(&*self)
    }
}

impl Sdf for &dyn Sdf {
    fn sdf(&self, point: Vec3) -> f32 {
        (*self).sdf(point)
    }
    fn aabb(&self) -> Option<Aabb3d> {
        (*self).aabb()
    }
}

// impl Sdf for Box<dyn Sdf> {
//     fn sdf(&self, point: Vec3) -> f32 {
//         (&*self).sdf(point)
//     }
//     fn aabb(&self) -> Option<Aabb3d> {
//         (&*self).aabb()
//     }
// }

impl Sdf for Sphere {
    fn sdf(&self, point: Vec3) -> f32 {
        point.length() - self.radius
    }
    fn aabb(&self) -> Option<Aabb3d> {
        Some(Aabb3d { min: Vec3A::splat(-self.radius), max: Vec3A::splat(self.radius) })
    }
}

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
        // xz using both radii and y using only the minor radius (the radius of the tube).
        Some(Aabb3d {
            min: Vec3A::new(-total_radii, -self.minor_radius, -total_radii),
            max: Vec3A::new(total_radii, self.minor_radius, total_radii),
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Blob;

impl Sdf for Blob {
    fn sdf(&self, point: Vec3) -> f32 {
        pub const PHI: f32 = 1.618034;
        let mut p = point.abs();
        if p.x < p.y.max(p.z) {
            p = Vec3::new(p.y, p.z, p.x);
        }
        if p.x < p.y.max(p.z) {
            p = Vec3::new(p.y, p.z, p.x);
        }
        let b = p
            .dot(Vec3::new(1.0, 1.0, 1.0).normalize())
            .max(Vec2::new(p.x, p.z).dot(Vec2::new(PHI + 1.0, 1.0).normalize()))
            .max(Vec2::new(p.y, p.x).dot(Vec2::new(1.0, PHI).normalize()))
            .max(Vec2::new(p.x, p.z).dot(Vec2::new(1.0, PHI).normalize()));
        let l = p.length();
        l - 1.5 - 0.2 * (1.5 / 2.0) * ((1.01 - b / l).sqrt() * (PI / 0.25)).min(PI).cos()
    }
    fn aabb(&self) -> Option<Aabb3d> {
        Some(Aabb3d { min: Vec3A::splat(-1.5), max: Vec3A::splat(1.5) })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Fractal;
impl Sdf for Fractal {
    fn sdf(&self, point: Vec3) -> f32 {
        let p0 = point;
        let mut p = Vec4::new(p0.x, p0.y, p0.z, 1.0);
        for _ in 0..8 {
            // p.xyz = mod(p.xyz-1.,2.)-1.;
            p.x = ((p.x - 1.0).rem_euclid(2.0)) - 1.0;
            p.y = ((p.y - 1.0).rem_euclid(2.0)) - 1.0;
            p.z = ((p.z - 1.0).rem_euclid(2.0)) - 1.0;
            let d = p.truncate().dot(p.truncate());
            p *= 1.4 / d;
        }
        let xz = Vec2::new(p.x, p.z);
        (xz / p.w).length() * 0.25
    }
    fn aabb(&self) -> Option<Aabb3d> {
        None
    }
}
