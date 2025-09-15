use std::sync::Arc;

use bevy::prelude::*;
use bevy_math::bounding::{Aabb3d, BoundingVolume};

use super::{Sdf, ops};
use crate::sdf::primitive;

#[derive(Debug, Clone, Reflect)]
// #[reflect(from_reflect = false)]
pub enum SdfNode {
    // Primitives
    Sphere { radius: f32 },
    Torus { major_radius: f32, minor_radius: f32 },
    Cuboid { size: Vec3 },
    RoundedBox { size: Vec3, radius: f32 },
    Ellipsoid { radii: Vec3 },
    Octahedron { size: f32 },
    HexagonalPrism { size: Vec3 },
    Pyramid { height: f32, base_size: Vec2 },
    Plane { normal: Vec3, distance: f32 },
    Cylinder { start: Vec3, end: Vec3, radius: f32 },
    Capsule { start: Vec3, end: Vec3, radius: f32 },
    Cone { a: Vec3, b: Vec3, radius_a: f32, radius_b: f32 },
    Triangle { v0: Vec3, v1: Vec3, v2: Vec3 },

    // Unary ops
    Translate { by: Vec3, child: Arc<SdfNode> },
    Rotate { by: Quat, child: Arc<SdfNode> },
    Scale { by: Vec3, child: Arc<SdfNode> },
    Round { radius: f32, child: Arc<SdfNode> },

    // Binary ops
    Union { a: Arc<SdfNode>, b: Arc<SdfNode> },
    Intersection { a: Arc<SdfNode>, b: Arc<SdfNode> },
    Subtraction { a: Arc<SdfNode>, b: Arc<SdfNode> },
    SmoothUnion { a: Arc<SdfNode>, b: Arc<SdfNode>, k: f32 },
    SmoothIntersection { a: Arc<SdfNode>, b: Arc<SdfNode>, k: f32 },
    SmoothSubtraction { a: Arc<SdfNode>, b: Arc<SdfNode>, k: f32 },
    Xor { a: Arc<SdfNode>, b: Arc<SdfNode> },
}

impl Default for SdfNode {
    fn default() -> Self {
        Self::Sphere { radius: 1.0 }
    }
}

impl Sdf for SdfNode {
    fn sdf(&self, point: Vec3) -> f32 {
        match self {
            // Primitives
            SdfNode::Sphere { radius } => primitive::Sphere { radius: *radius }.sdf(point),
            SdfNode::Torus { major_radius, minor_radius } => {
                primitive::Torus { major_radius: *major_radius, minor_radius: *minor_radius }
                    .sdf(point)
            },
            SdfNode::Cuboid { size } => primitive::Cuboid { size: *size }.sdf(point),
            SdfNode::RoundedBox { size, radius } => {
                primitive::RoundedBox { size: *size, radius: *radius }.sdf(point)
            },
            SdfNode::Ellipsoid { radii } => primitive::Ellipsoid { radii: *radii }.sdf(point),
            SdfNode::Octahedron { size } => primitive::Octahedron { size: *size }.sdf(point),
            SdfNode::HexagonalPrism { size } => {
                primitive::HexagonalPrism { size: *size }.sdf(point)
            },
            SdfNode::Pyramid { height, base_size } => {
                primitive::Pyramid { height: *height, base_size: *base_size }.sdf(point)
            },
            SdfNode::Plane { normal, distance } => {
                primitive::Plane { normal: *normal, distance: *distance }.sdf(point)
            },
            SdfNode::Cylinder { start, end, radius } => {
                primitive::Cylinder { start: *start, end: *end, radius: *radius }.sdf(point)
            },
            SdfNode::Capsule { start, end, radius } => {
                primitive::Capsule { start: *start, end: *end, radius: *radius }.sdf(point)
            },
            SdfNode::Cone { a, b, radius_a, radius_b } => {
                primitive::Cone { a: *a, b: *b, radius_a: *radius_a, radius_b: *radius_b }
                    .sdf(point)
            },
            SdfNode::Triangle { v0, v1, v2 } => {
                primitive::Triangle { v0: *v0, v1: *v1, v2: *v2 }.sdf(point)
            },

            // Unary ops (evaluate inline to avoid Reflect on generic ops)
            SdfNode::Translate { by, child } => child.sdf(point - *by),
            SdfNode::Rotate { by, child } => child.sdf(by.inverse() * point),
            SdfNode::Scale { by, child } => {
                let inv = Vec3::new(1.0 / by.x, 1.0 / by.y, 1.0 / by.z);
                child.sdf(point * inv) * by.x.min(by.y.min(by.z))
            },
            SdfNode::Round { radius, child } => child.sdf(point) - *radius,

            // Binary ops
            SdfNode::Union { a, b } => a.sdf(point).min(b.sdf(point)),
            SdfNode::Intersection { a, b } => a.sdf(point).max(b.sdf(point)),
            SdfNode::Subtraction { a, b } => (-a.sdf(point)).max(b.sdf(point)),
            SdfNode::SmoothUnion { a, b, k } => {
                let d1 = a.sdf(point);
                let d2 = b.sdf(point);
                let h = (0.5 + 0.5 * (d2 - d1) / *k).clamp(0.0, 1.0);
                d2 * (1.0 - h) + d1 * h - *k * h * (1.0 - h)
            },
            SdfNode::SmoothIntersection { a, b, k } => {
                let d1 = a.sdf(point);
                let d2 = b.sdf(point);
                let h = (0.5 - 0.5 * (d2 - d1) / *k).clamp(0.0, 1.0);
                d2 * (1.0 - h) + d1 * h + *k * h * (1.0 - h)
            },
            SdfNode::SmoothSubtraction { a, b, k } => {
                let d1 = a.sdf(point);
                let d2 = b.sdf(point);
                let h = (0.5 - 0.5 * (d2 + d1) / *k).clamp(0.0, 1.0);
                d2 * (1.0 - h) + (-d1) * h + *k * h * (1.0 - h)
            },
            SdfNode::Xor { a, b } => {
                let d1 = a.sdf(point);
                let d2 = b.sdf(point);
                d1.min(d2).max(-d1.max(d2))
            },
        }
    }

    fn aabb(&self) -> Option<Aabb3d> {
        match self {
            // Primitives
            SdfNode::Sphere { radius } => primitive::Sphere { radius: *radius }.aabb(),
            SdfNode::Torus { major_radius, minor_radius } => {
                primitive::Torus { major_radius: *major_radius, minor_radius: *minor_radius }.aabb()
            },
            SdfNode::Cuboid { size } => primitive::Cuboid { size: *size }.aabb(),
            SdfNode::RoundedBox { size, radius } => {
                primitive::RoundedBox { size: *size, radius: *radius }.aabb()
            },
            SdfNode::Ellipsoid { radii } => primitive::Ellipsoid { radii: *radii }.aabb(),
            SdfNode::Octahedron { size } => primitive::Octahedron { size: *size }.aabb(),
            SdfNode::HexagonalPrism { size } => primitive::HexagonalPrism { size: *size }.aabb(),
            SdfNode::Pyramid { height, base_size } => {
                primitive::Pyramid { height: *height, base_size: *base_size }.aabb()
            },
            SdfNode::Plane { .. } => None,
            SdfNode::Cylinder { start, end, radius } => {
                primitive::Cylinder { start: *start, end: *end, radius: *radius }.aabb()
            },
            SdfNode::Capsule { start, end, radius } => {
                primitive::Capsule { start: *start, end: *end, radius: *radius }.aabb()
            },
            SdfNode::Cone { a, b, radius_a, radius_b } => {
                primitive::Cone { a: *a, b: *b, radius_a: *radius_a, radius_b: *radius_b }.aabb()
            },
            SdfNode::Triangle { v0, v1, v2 } => {
                primitive::Triangle { v0: *v0, v1: *v1, v2: *v2 }.aabb()
            },

            // Unary ops (inline AABB)
            SdfNode::Translate { by, child } => child.aabb().map(|aabb| aabb.translated_by(*by)),
            SdfNode::Rotate { by, child } => child.aabb().map(|aabb| aabb.rotated_by(*by)),
            SdfNode::Scale { by, child } => child.aabb().map(|aabb| aabb.scale_around_center(*by)),
            SdfNode::Round { radius, child } => child.aabb().map(|aabb| {
                let expansion = Vec3A::splat(*radius);
                Aabb3d { min: aabb.min - expansion, max: aabb.max + expansion }
            }),

            // Binary ops
            SdfNode::Union { a, b } | SdfNode::Xor { a, b } => match (a.aabb(), b.aabb()) {
                (Some(a), Some(b)) => Some(Aabb3d { min: a.min.min(b.min), max: a.max.max(b.max) }),
                (Some(a), None) | (None, Some(a)) => Some(a),
                (None, None) => None,
            },
            SdfNode::Intersection { a, b } => match (a.aabb(), b.aabb()) {
                (Some(a), Some(b)) => {
                    let min = a.min.max(b.min);
                    let max = a.max.min(b.max);
                    if min.x <= max.x && min.y <= max.y && min.z <= max.z {
                        Some(Aabb3d { min, max })
                    } else {
                        None
                    }
                },
                _ => None,
            },
            SdfNode::Subtraction { a: _, b } => b.aabb(),
            SdfNode::SmoothUnion { a, b, k } => match (a.aabb(), b.aabb()) {
                (Some(a), Some(b)) => {
                    let expansion = Vec3A::splat(*k);
                    Some(Aabb3d {
                        min: a.min.min(b.min) - expansion,
                        max: a.max.max(b.max) + expansion,
                    })
                },
                (Some(a), None) | (None, Some(a)) => Some(a),
                (None, None) => None,
            },
            SdfNode::SmoothIntersection { a, b, k } => match (a.aabb(), b.aabb()) {
                (Some(a), Some(b)) => {
                    let min = a.min.max(b.min);
                    let max = a.max.min(b.max);
                    if min.x <= max.x && min.y <= max.y && min.z <= max.z {
                        let expansion = Vec3A::splat(*k);
                        Some(Aabb3d { min: min - expansion, max: max + expansion })
                    } else {
                        None
                    }
                },
                _ => None,
            },
            SdfNode::SmoothSubtraction { a: _, b, k } => b.aabb().map(|aabb| {
                let expansion = Vec3A::splat(*k);
                Aabb3d { min: aabb.min - expansion, max: aabb.max + expansion }
            }),
        }
    }
}
