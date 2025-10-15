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
    app.register_type::<Arc<SdfNode>>();

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
    fn as_node(&self) -> SdfNode;

    // -- ops --

    /// Scale this SDF from world space into voxel space.
    fn to_voxel_space(self) -> ops::Scale<Self>
    where
        Self: Sized + Clone + Default,
    {
        ops::Scale { scale: 1.0 / crate::voxel::GRID_SCALE, primitive: self }
    }

    /// Scale this SDF from voxel space into world space.
    fn to_world_space(self) -> ops::Scale<Self>
    where
        Self: Sized + Clone + Default,
    {
        ops::Scale { scale: crate::voxel::GRID_SCALE, primitive: self }
    }

    /// Translate this SDF by an amount of voxel cells.
    fn voxel_translate(self, by: IVec3) -> ops::Translate<Self>
    where
        Self: Sized + Clone + Default,
    {
        ops::Translate { translate: by.as_vec3() * crate::voxel::GRID_SCALE, primitive: self }
    }

    // isometry
    fn translate(self, by: Vec3) -> ops::Translate<Self>
    where
        Self: Sized + Clone,
    {
        ops::Translate { translate: by, primitive: self }
    }
    fn rotate(self, by: Quat) -> ops::Rotate<Self>
    where
        Self: Sized + Clone + Default,
    {
        ops::Rotate { rotate: by, primitive: self }
    }
    fn scale(self, by: Vec3) -> ops::Scale<Self>
    where
        Self: Sized + Clone + Default,
    {
        ops::Scale { scale: by, primitive: self }
    }

    // combiners
    fn union<O: Sdf>(self, other: O) -> ops::Union<Self, O>
    where
        Self: Sized + Clone + Default,
        O: Clone + Default,
    {
        ops::Union { a: self, b: other }
    }
    fn intersection<O: Sdf>(self, other: O) -> ops::Intersection<Self, O>
    where
        Self: Sized + Clone + Default,
        O: Clone + Default,
    {
        ops::Intersection { a: self, b: other }
    }
    fn subtraction<O: Sdf>(self, other: O) -> ops::Subtraction<Self, O>
    where
        Self: Sized + Clone + Default,
        O: Clone + Default,
    {
        ops::Subtraction { a: self, b: other }
    }

    // smooth combiners
    fn smooth_union<O: Sdf>(self, other: O, smooth: f32) -> ops::SmoothUnion<Self, O>
    where
        Self: Sized + Clone + Default,
        O: Clone + Default,
    {
        ops::SmoothUnion { a: self, b: other, k: smooth }
    }
    fn smooth_intersection<O: Sdf>(self, other: O, smooth: f32) -> ops::SmoothIntersection<Self, O>
    where
        Self: Sized + Clone + Default,
        O: Clone + Default,
    {
        ops::SmoothIntersection { a: self, b: other, k: smooth }
    }
    fn smooth_subtraction<O: Sdf>(self, other: O, smooth: f32) -> ops::SmoothSubtraction<Self, O>
    where
        Self: Sized + Clone + Default,
        O: Clone + Default,
    {
        ops::SmoothSubtraction { a: self, b: other, k: smooth }
    }

    // misc
    fn round(self, radius: f32) -> ops::Round<Self>
    where
        Self: Sized + Clone + Default,
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

    fn as_node(&self) -> SdfNode {
        S::as_node(&*self)
    }
}

impl Sdf for Box<dyn Sdf + Send + Sync> {
    fn sdf(&self, point: Vec3) -> f32 {
        let s: &(dyn Sdf + Send + Sync) = &*self;
        s.sdf(point)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        let s: &(dyn Sdf + Send + Sync) = &*self;
        s.aabb()
    }

    fn as_node(&self) -> SdfNode {
        let s: &(dyn Sdf + Send + Sync) = &*self;
        s.as_node()
    }
}

impl<S: Sdf> Sdf for std::sync::Arc<S> {
    fn sdf(&self, point: Vec3) -> f32 {
        S::sdf(&*self, point)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        S::aabb(&*self)
    }

    fn as_node(&self) -> SdfNode {
        S::as_node(&*self)
    }
}

impl<'a, S: Sdf> Sdf for &'a S {
    fn sdf(&self, point: Vec3) -> f32 {
        S::sdf(self, point)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        S::aabb(self)
    }

    fn as_node(&self) -> SdfNode {
        S::as_node(self)
    }
}

impl Sdf for &dyn Sdf {
    fn sdf(&self, point: Vec3) -> f32 {
        (*self).sdf(point)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        (*self).aabb()
    }

    fn as_node(&self) -> SdfNode {
        (*self).as_node()
    }
}

impl Sdf for &(dyn Sdf + Send) {
    fn sdf(&self, point: Vec3) -> f32 {
        (*self).sdf(point)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        (*self).aabb()
    }

    fn as_node(&self) -> SdfNode {
        (*self).as_node()
    }
}

impl Sdf for &(dyn Sdf + Send + Sync) {
    fn sdf(&self, point: Vec3) -> f32 {
        (*self).sdf(point)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        (*self).aabb()
    }

    fn as_node(&self) -> SdfNode {
        (*self).as_node()
    }
}
