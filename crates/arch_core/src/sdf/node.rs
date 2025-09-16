use std::sync::Arc;

// use bevy::reflect::Reflect;
use bevy::prelude::*;
use bevy_math::bounding::{Aabb3d, BoundingVolume};

use super::{Sdf, ops};
use crate::sdf;

#[derive(Debug, Clone, Reflect)]
pub enum SdfNode {
    // Primitives
    Sphere(sdf::Sphere),
    Torus(sdf::Torus),
    Cuboid(sdf::Cuboid),
    RoundedBox(sdf::RoundedBox),
    Ellipsoid(sdf::Ellipsoid),
    Octahedron(sdf::Octahedron),
    HexagonalPrism(sdf::HexagonalPrism),
    Pyramid(sdf::Pyramid),
    Plane(sdf::Plane),
    Cylinder(sdf::primitive::Cylinder),
    Capsule(sdf::Capsule),
    Cone(sdf::primitive::Cone),
    Triangle(sdf::Triangle),

    // Unary ops
    Translate(ops::Translate<Arc<SdfNode>>),
    Rotate(ops::Rotate<Arc<SdfNode>>),
    Scale(ops::Scale<Arc<SdfNode>>),
    Round(ops::Round<Arc<SdfNode>>),

    // Binary ops
    Union(ops::Union<Arc<SdfNode>, Arc<SdfNode>>),
    Intersection(ops::Intersection<Arc<SdfNode>, Arc<SdfNode>>),
    Subtraction(ops::Subtraction<Arc<SdfNode>, Arc<SdfNode>>),
    SmoothUnion(ops::SmoothUnion<Arc<SdfNode>, Arc<SdfNode>>),
    SmoothIntersection(ops::SmoothIntersection<Arc<SdfNode>, Arc<SdfNode>>),
    SmoothSubtraction(ops::SmoothSubtraction<Arc<SdfNode>, Arc<SdfNode>>),
    Xor(ops::Xor<Arc<SdfNode>, Arc<SdfNode>>),
}

impl Default for SdfNode {
    fn default() -> Self {
        Self::Sphere(sdf::Sphere { radius: 0.5 })
    }
}

impl Sdf for Arc<SdfNode> {
    fn sdf(&self, point: Vec3) -> f32 {
        SdfNode::sdf(self, point)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        SdfNode::aabb(self)
    }
}

impl Sdf for SdfNode {
    fn sdf(&self, point: Vec3) -> f32 {
        match self {
            SdfNode::Sphere(sphere) => sphere.sdf(point),
            SdfNode::Torus(torus) => torus.sdf(point),
            SdfNode::Cuboid(cuboid) => cuboid.sdf(point),
            SdfNode::RoundedBox(rounded_box) => rounded_box.sdf(point),
            SdfNode::Ellipsoid(ellipsoid) => ellipsoid.sdf(point),
            SdfNode::Octahedron(octahedron) => octahedron.sdf(point),
            SdfNode::HexagonalPrism(hexagonal_prism) => hexagonal_prism.sdf(point),
            SdfNode::Pyramid(pyramid) => pyramid.sdf(point),
            SdfNode::Plane(plane) => plane.sdf(point),
            SdfNode::Cylinder(cylinder) => cylinder.sdf(point),
            SdfNode::Capsule(capsule) => capsule.sdf(point),
            SdfNode::Cone(cone) => cone.sdf(point),
            SdfNode::Triangle(triangle) => triangle.sdf(point),
            SdfNode::Translate(translate) => translate.sdf(point),
            SdfNode::Rotate(rotate) => rotate.sdf(point),
            SdfNode::Scale(scale) => scale.sdf(point),
            SdfNode::Round(round) => round.sdf(point),
            SdfNode::Union(union) => union.sdf(point),
            SdfNode::Intersection(intersection) => intersection.sdf(point),
            SdfNode::Subtraction(subtraction) => subtraction.sdf(point),
            SdfNode::SmoothUnion(smooth_union) => smooth_union.sdf(point),
            SdfNode::SmoothIntersection(smooth_intersection) => smooth_intersection.sdf(point),
            SdfNode::SmoothSubtraction(smooth_subtraction) => smooth_subtraction.sdf(point),
            SdfNode::Xor(xor) => xor.sdf(point),
        }
    }

    fn aabb(&self) -> Option<Aabb3d> {
        match self {
            SdfNode::Sphere(sphere) => sphere.aabb(),
            SdfNode::Torus(torus) => torus.aabb(),
            SdfNode::Cuboid(cuboid) => cuboid.aabb(),
            SdfNode::RoundedBox(rounded_box) => rounded_box.aabb(),
            SdfNode::Ellipsoid(ellipsoid) => ellipsoid.aabb(),
            SdfNode::Octahedron(octahedron) => octahedron.aabb(),
            SdfNode::HexagonalPrism(hexagonal_prism) => hexagonal_prism.aabb(),
            SdfNode::Pyramid(pyramid) => pyramid.aabb(),
            SdfNode::Plane(plane) => plane.aabb(),
            SdfNode::Cylinder(cylinder) => cylinder.aabb(),
            SdfNode::Capsule(capsule) => capsule.aabb(),
            SdfNode::Cone(cone) => cone.aabb(),
            SdfNode::Triangle(triangle) => triangle.aabb(),
            SdfNode::Translate(translate) => translate.aabb(),
            SdfNode::Rotate(rotate) => rotate.aabb(),
            SdfNode::Scale(scale) => scale.aabb(),
            SdfNode::Round(round) => round.aabb(),
            SdfNode::Union(union) => union.aabb(),
            SdfNode::Intersection(intersection) => intersection.aabb(),
            SdfNode::Subtraction(subtraction) => subtraction.aabb(),
            SdfNode::SmoothUnion(smooth_union) => smooth_union.aabb(),
            SdfNode::SmoothIntersection(smooth_intersection) => smooth_intersection.aabb(),
            SdfNode::SmoothSubtraction(smooth_subtraction) => smooth_subtraction.aabb(),
            SdfNode::Xor(xor) => xor.aabb(),
        }
    }
}
