//! SDF (Signed distance functions) for voxel rasterization.

use std::f32::consts::PI;

pub use bevy::math::primitives::{Sphere, Torus};
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

pub mod ops;
pub mod primitive;
pub mod voxel_rasterize;

pub use primitive::*;

pub trait Sdf: Send + Sync {
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

impl<'a, S: Sdf> Sdf for &'a S {
    fn sdf(&self, point: Vec3) -> f32 {
        S::sdf(self, point)
    }
    fn aabb(&self) -> Option<Aabb3d> {
        S::aabb(self)
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

impl Sdf for &(dyn Sdf + Send) {
    fn sdf(&self, point: Vec3) -> f32 {
        (*self).sdf(point)
    }
    fn aabb(&self) -> Option<Aabb3d> {
        (*self).aabb()
    }
}

impl Sdf for &(dyn Sdf + Send + Sync) {
    fn sdf(&self, point: Vec3) -> f32 {
        (*self).sdf(point)
    }
    fn aabb(&self) -> Option<Aabb3d> {
        (*self).aabb()
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