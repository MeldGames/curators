use std::sync::Arc;

use bevy::reflect::Reflect;
// use bevy::prelude::*;
use bevy_math::bounding::{Aabb3d, BoundingVolume};

use super::{Sdf, ops};
use crate::sdf::{self, primitive};

#[derive(Debug, Clone, Reflect)]
// #[reflect(from_reflect = false)]
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
    Cylinder(sdf::Cylinder),
    Capsule(sdf::Capsule),
    Cone(sdf::Cone),
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

// Prints out a string of Rust code that reconstructs this SdfNode as Rust code
// using the primitives directly. The output is written to the provided `f`
// (e.g., &mut String or std::fmt::Write).
// pub fn print_as_rust_code(&self, f: &mut dyn std::fmt::Write) ->
// std::fmt::Result { match self {
// SdfNode::Cuboid { size } => {
// write!(f, "Cuboid::new(Vec3::new({}, {}, {}))", size.x, size.y, size.z)
// },
// SdfNode::RoundedBox { size, radius } => {
// write!(f, "RoundedBox::new(Vec3::new({}, {}, {}), {})", size.x, size.y,
// size.z, radius) },
// SdfNode::Ellipsoid { radii } => {
// write!(f, "Ellipsoid::new(Vec3::new({}, {}, {}))", radii.x, radii.y, radii.z)
// },
// SdfNode::Octahedron { size } => {
// write!(f, "Octahedron::new({})", size)
// },
// SdfNode::HexagonalPrism { size } => {
// write!(f, "HexagonalPrism::new(Vec3::new({}, {}, {}))", size.x, size.y,
// size.z) },
// SdfNode::Pyramid { size } => {
// write!(f, "Pyramid::new(Vec3::new({}, {}, {}))", size.x, size.y, size.z)
// },
// SdfNode::Plane { normal, d } => {
// write!(f, "Plane::new(Vec3::new({}, {}, {}), {})", normal.x, normal.y,
// normal.z, d) },
// SdfNode::Cylinder { radius, height } => {
// write!(f, "Cylinder::new({}, {})", radius, height)
// },
// SdfNode::Capsule { start, end, radius } => {
// write!(
// f,
// "Capsule::new(Vec3::new({}, {}, {}), Vec3::new({}, {}, {}), {})",
// start.x, start.y, start.z, end.x, end.y, end.z, radius
// )
// },
// SdfNode::Cone { height, radius1, radius2 } => {
// write!(f, "Cone::new({}, {}, {})", height, radius1, radius2)
// },
// SdfNode::Triangle { a, b, c } => {
// write!(
// f,
// "Triangle::new(Vec3::new({}, {}, {}), Vec3::new({}, {}, {}), Vec3::new({},
// {}, \ {}))",
// a.x, a.y, a.z, b.x, b.y, b.z, c.x, c.y, c.z
// )
// },
// SdfNode::Blob => {
// write!(f, "Blob")
// },
// SdfNode::Fractal => {
// write!(f, "Fractal")
// },
// SdfNode::Translate { by, primitive } => {
// write!(f, "(")?;
// primitive.print_as_rust_code(f)?;
// write!(f, ").translate(Vec3::new({}, {}, {}))", by.x, by.y, by.z)
// },
// SdfNode::Rotate { by, primitive } => {
// write!(f, "(")?;
// primitive.print_as_rust_code(f)?;
// write!(f, ").rotate(Quat::from_xyzw({}, {}, {}, {}))", by.x, by.y, by.z,
// by.w) },
// SdfNode::Scale { by, primitive } => {
// write!(f, "(")?;
// primitive.print_as_rust_code(f)?;
// write!(f, ").scale(Vec3::new({}, {}, {}))", by.x, by.y, by.z)
// },
// SdfNode::Union { a, b } => {
// write!(f, "(")?;
// a.print_as_rust_code(f)?;
// write!(f, ").union(")?;
// b.print_as_rust_code(f)?;
// write!(f, ")")
// },
// SdfNode::Intersection { a, b } => {
// write!(f, "(")?;
// a.print_as_rust_code(f)?;
// write!(f, ").intersection(")?;
// b.print_as_rust_code(f)?;
// write!(f, ")")
// },
// SdfNode::Subtraction { a, b } => {
// write!(f, "(")?;
// a.print_as_rust_code(f)?;
// write!(f, ").subtraction(")?;
// b.print_as_rust_code(f)?;
// write!(f, ")")
// },
// SdfNode::SmoothUnion { a, b, k } => {
// write!(f, "(")?;
// a.print_as_rust_code(f)?;
// write!(f, ").smooth_union(")?;
// b.print_as_rust_code(f)?;
// write!(f, ", {})", k)
// },
// SdfNode::SmoothIntersection { a, b, k } => {
// write!(f, "(")?;
// a.print_as_rust_code(f)?;
// write!(f, ").smooth_intersection(")?;
// b.print_as_rust_code(f)?;
// write!(f, ", {})", k)
// },
// SdfNode::SmoothSubtraction { a, b, k } => {
// write!(f, "(")?;
// a.print_as_rust_code(f)?;
// write!(f, ").smooth_subtraction(")?;
// b.print_as_rust_code(f)?;
// write!(f, ", {})", k)
// },
// SdfNode::Round { primitive, radius } => {
// write!(f, "(")?;
// primitive.print_as_rust_code(f)?;
// write!(f, ").round({})", radius)
// },
// }
// }
