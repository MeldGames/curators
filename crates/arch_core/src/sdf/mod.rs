//! SDF (Signed distance functions) for voxel rasterization.

use std::f32::consts::PI;
use std::fmt::Debug;
use std::sync::Arc;

pub use bevy::math::primitives::{Sphere, Torus};
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

pub mod node;
pub mod ops;
pub mod primitive;
pub mod voxel_rasterize;

pub use node::SdfNode;
pub use primitive::*;

/// Register SDF reflection types for editor/inspector usage
pub fn register_sdf_reflect_types(app: &mut App) {
    app.register_type::<SdfNode>();
    // Primitives
    app.register_type::<Cuboid>();
    app.register_type::<RoundedBox>();
    app.register_type::<Ellipsoid>();
    app.register_type::<Octahedron>();
    app.register_type::<HexagonalPrism>();
    app.register_type::<Pyramid>();
    app.register_type::<Plane>();
    app.register_type::<primitive::Cylinder>();
    app.register_type::<Capsule>();
    app.register_type::<primitive::Cone>();
    app.register_type::<Triangle>();

    // Ops
    app.register_type::<ops::Translate<Arc<SdfNode>>>()
        .register_type::<ops::Rotate<Arc<SdfNode>>>()
        .register_type::<ops::Scale<Arc<SdfNode>>>()
        .register_type::<ops::Round<Arc<SdfNode>>>()
        .register_type::<ops::Union<Arc<SdfNode>, Arc<SdfNode>>>()
        .register_type::<ops::Intersection<Arc<SdfNode>, Arc<SdfNode>>>()
        .register_type::<ops::Subtraction<Arc<SdfNode>, Arc<SdfNode>>>()
        .register_type::<ops::SmoothUnion<Arc<SdfNode>, Arc<SdfNode>>>()
        .register_type::<ops::SmoothIntersection<Arc<SdfNode>, Arc<SdfNode>>>()
        .register_type::<ops::SmoothSubtraction<Arc<SdfNode>, Arc<SdfNode>>>()
        .register_type::<ops::Xor<Arc<SdfNode>, Arc<SdfNode>>>();
}

pub trait Sdf: Send + Sync + Debug {
    fn sdf(&self, point: Vec3) -> f32;
    fn aabb(&self) -> Option<Aabb3d>;

    // -- ops --

    // isometry
    fn translate(self, by: Vec3) -> ops::Translate<Self>
    where
        Self: Sized,
    {
        ops::Translate { translate: by, primitive: self }
    }
    fn rotate(self, by: Quat) -> ops::Rotate<Self>
    where
        Self: Sized,
    {
        ops::Rotate { rotate: by, primitive: self }
    }
    fn scale(self, by: Vec3) -> ops::Scale<Self>
    where
        Self: Sized,
    {
        ops::Scale { scale: by, primitive: self }
    }

    // combiners
    fn union<O: Sdf>(self, other: O) -> ops::Union<Self, O>
    where
        Self: Sized,
    {
        ops::Union { a: self, b: other }
    }
    fn intersection<O: Sdf>(self, other: O) -> ops::Intersection<Self, O>
    where
        Self: Sized,
    {
        ops::Intersection { a: self, b: other }
    }
    fn subtraction<O: Sdf>(self, other: O) -> ops::Subtraction<Self, O>
    where
        Self: Sized,
    {
        ops::Subtraction { a: self, b: other }
    }

    // smooth combiners
    fn smooth_union<O: Sdf>(self, other: O, smooth: f32) -> ops::SmoothUnion<Self, O>
    where
        Self: Sized,
    {
        ops::SmoothUnion { a: self, b: other, k: smooth }
    }
    fn smooth_intersection<O: Sdf>(self, other: O, smooth: f32) -> ops::SmoothIntersection<Self, O>
    where
        Self: Sized,
    {
        ops::SmoothIntersection { a: self, b: other, k: smooth }
    }
    fn smooth_subtraction<O: Sdf>(self, other: O, smooth: f32) -> ops::SmoothSubtraction<Self, O>
    where
        Self: Sized,
    {
        ops::SmoothSubtraction { a: self, b: other, k: smooth }
    }

    // misc
    fn round(self, radius: f32) -> ops::Round<Self>
    where
        Self: Sized,
    {
        ops::Round { primitive: self, radius }
    }
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

#[derive(Clone, Copy, Debug, Reflect)]
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

#[derive(Clone, Copy, Debug, Reflect)]
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
