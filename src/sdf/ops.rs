use crate::sdf::Sdf;
use bevy::prelude::*;
use bevy_math::bounding::{Aabb3d, BoundingVolume};


// Helper functions
fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.max(min).min(max)
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

/// Scale the underlying primitive.
///
/// Non uniform scaling is supported, but may cause some issues.
#[derive(Debug, Clone)]
pub struct Scale<P: Sdf> {
    pub primitive: P,
    pub scale: Vec3,
}

impl<P: Sdf> Sdf for Scale<P> {
    fn sdf(&self, point: Vec3) -> f32 {
        self.primitive.sdf(point / self.scale) * self.scale.x.min(self.scale.y.min(self.scale.z))
    }

    fn aabb(&self) -> Option<bevy_math::bounding::Aabb3d> {
        self.primitive.aabb().map(|aabb| aabb.scale_around_center(self.scale))
    }
}

#[derive(Debug, Clone)]
pub struct Isometry<P: Sdf> {
    pub rotation: Quat,
    pub translation: Vec3,
    pub primitive: P,
}

impl<P: Sdf> Sdf for Isometry<P> {
    fn sdf(&self, point: Vec3) -> f32 {
        let inverted_point = self.rotation.inverse() * (point - self.translation);
        self.primitive.sdf(inverted_point)
    }

    fn aabb(&self) -> Option<bevy_math::bounding::Aabb3d> {
        self.primitive.aabb().map(|aabb| aabb.transformed_by(self.translation, self.rotation))
    }
}


#[derive(Debug, Clone)]
pub struct Twist<P: Sdf> {
    pub strength: f32,
    pub primitive: P,
}

impl<P: Sdf> Sdf for Twist<P> {
    fn sdf(&self, point: Vec3) -> f32 {
        let c = (self.strength * point.y).cos();
        let s = (self.strength * point.y).sin();
        
        let m = mat2(vec2(c, -s), vec2(s, c));
        
        let rotated_xz = m.mul_vec2(vec2(point.x, point.z));
        let rotated_point = Vec3::new(rotated_xz.x, point.y, rotated_xz.y);
        
        self.primitive.sdf(rotated_point)
    }

    fn aabb(&self) -> Option<bevy_math::bounding::Aabb3d> {
        self.primitive.aabb().map(|aabb| {
            Aabb3d {
                min: Vec3A::new(
                    aabb.min.x.min(aabb.min.z),
                    aabb.min.y,
                    aabb.min.x.min(aabb.min.z),
                ),
                max: Vec3A::new(
                    aabb.max.x.max(aabb.max.z),
                    aabb.max.y,
                    aabb.max.x.max(aabb.max.z),
                ),
            }
        })
    }
}

// Union operation
#[derive(Debug, Clone)]
pub struct Union<A: Sdf, B: Sdf> {
    pub a: A,
    pub b: B,
}

impl<A: Sdf, B: Sdf> Sdf for Union<A, B> {
    fn sdf(&self, point: Vec3) -> f32 {
        let d1 = self.a.sdf(point);
        let d2 = self.b.sdf(point);
        d1.min(d2)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        match (self.a.aabb(), self.b.aabb()) {
            (Some(a), Some(b)) => Some(Aabb3d {
                min: a.min.min(b.min),
                max: a.max.max(b.max),
            }),
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        }
    }
}

// Subtraction operation
#[derive(Debug, Clone)]
pub struct Subtraction<A: Sdf, B: Sdf> {
    pub a: A,
    pub b: B,
}

impl<A: Sdf, B: Sdf> Sdf for Subtraction<A, B> {
    fn sdf(&self, point: Vec3) -> f32 {
        let d1 = self.a.sdf(point);
        let d2 = self.b.sdf(point);
        (-d1).max(d2)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        // Subtraction can only shrink the result, so use the second operand's bounds
        self.b.aabb()
    }
}

// Intersection operation
#[derive(Debug, Clone)]
pub struct Intersection<A: Sdf, B: Sdf> {
    pub a: A,
    pub b: B,
}

impl<A: Sdf, B: Sdf> Sdf for Intersection<A, B> {
    fn sdf(&self, point: Vec3) -> f32 {
        let d1 = self.a.sdf(point);
        let d2 = self.b.sdf(point);
        d1.max(d2)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        match (self.a.aabb(), self.b.aabb()) {
            (Some(a), Some(b)) => {
                let min = a.min.max(b.min);
                let max = a.max.min(b.max);
                if min.x <= max.x && min.y <= max.y && min.z <= max.z {
                    Some(Aabb3d { min, max })
                } else {
                    None // No intersection
                }
            }
            _ => None,
        }
    }
}

// XOR operation
#[derive(Debug, Clone)]
pub struct Xor<A: Sdf, B: Sdf> {
    pub a: A,
    pub b: B,
}

impl<A: Sdf, B: Sdf> Sdf for Xor<A, B> {
    fn sdf(&self, point: Vec3) -> f32 {
        let d1 = self.a.sdf(point);
        let d2 = self.b.sdf(point);
        d1.min(d2).max(-d1.max(d2))
    }

    fn aabb(&self) -> Option<Aabb3d> {
        // XOR bounds are complex, use union as approximation
        match (self.a.aabb(), self.b.aabb()) {
            (Some(a), Some(b)) => Some(Aabb3d {
                min: a.min.min(b.min),
                max: a.max.max(b.max),
            }),
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        }
    }
}

// Smooth Union operation
#[derive(Debug, Clone)]
pub struct SmoothUnion<A: Sdf, B: Sdf> {
    pub a: A,
    pub b: B,
    pub k: f32,
}

impl<A: Sdf, B: Sdf> Sdf for SmoothUnion<A, B> {
    fn sdf(&self, point: Vec3) -> f32 {
        let d1 = self.a.sdf(point);
        let d2 = self.b.sdf(point);
        let h = clamp(0.5 + 0.5 * (d2 - d1) / self.k, 0.0, 1.0);
        mix(d2, d1, h) - self.k * h * (1.0 - h)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        match (self.a.aabb(), self.b.aabb()) {
            (Some(a), Some(b)) => {
                // Smooth union can extend beyond both shapes due to blending
                let expansion = Vec3A::splat(self.k);
                Some(Aabb3d {
                    min: a.min.min(b.min) - expansion,
                    max: a.max.max(b.max) + expansion,
                })
            }
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        }
    }
}

// Smooth Subtraction operation
#[derive(Debug, Clone)]
pub struct SmoothSubtraction<A: Sdf, B: Sdf> {
    pub a: A,
    pub b: B,
    pub k: f32,
}

impl<A: Sdf, B: Sdf> Sdf for SmoothSubtraction<A, B> {
    fn sdf(&self, point: Vec3) -> f32 {
        let d1 = self.a.sdf(point);
        let d2 = self.b.sdf(point);
        let h = clamp(0.5 - 0.5 * (d2 + d1) / self.k, 0.0, 1.0);
        mix(d2, -d1, h) + self.k * h * (1.0 - h)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        // Smooth subtraction can extend the result slightly
        self.b.aabb().map(|aabb| {
            let expansion = Vec3A::splat(self.k);
            Aabb3d { min: aabb.min - expansion, max: aabb.max + expansion }
        })
    }
}

// Smooth Intersection operation
#[derive(Debug, Clone)]
pub struct SmoothIntersection<A: Sdf, B: Sdf> {
    pub a: A,
    pub b: B,
    pub k: f32,
}

impl<A: Sdf, B: Sdf> Sdf for SmoothIntersection<A, B> {
    fn sdf(&self, point: Vec3) -> f32 {
        let d1 = self.a.sdf(point);
        let d2 = self.b.sdf(point);
        let h = clamp(0.5 - 0.5 * (d2 - d1) / self.k, 0.0, 1.0);
        mix(d2, d1, h) + self.k * h * (1.0 - h)
    }

    fn aabb(&self) -> Option<Aabb3d> {
        match (self.a.aabb(), self.b.aabb()) {
            (Some(a), Some(b)) => {
                // Smooth intersection can extend slightly beyond the intersection
                let min = a.min.max(b.min);
                let max = a.max.min(b.max);
                if min.x <= max.x && min.y <= max.y && min.z <= max.z {
                    let expansion = Vec3A::splat(self.k);
                    Some(Aabb3d { min: min - expansion, max: max + expansion})
                } else {
                    None // No intersection
                }
            }
            _ => None,
        }
    }
}

// Round operation
#[derive(Debug, Clone)]
pub struct Round<P: Sdf> {
    pub primitive: P,
    pub radius: f32,
}

impl<P: Sdf> Sdf for Round<P> {
    fn sdf(&self, point: Vec3) -> f32 {
        self.primitive.sdf(point) - self.radius
    }

    fn aabb(&self) -> Option<Aabb3d> {
        self.primitive.aabb().map(|aabb| {
            let expansion = Vec3A::splat(self.radius);
            Aabb3d { min: aabb.min - expansion, max: aabb.max + expansion }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_union() {
        let sphere1 = Isometry {
            translation: Vec3::new(-1.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            primitive: Sphere { radius: 1.0 },
        };
        let sphere2 = Isometry {
            translation: Vec3::new(1.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            primitive: Sphere { radius: 1.0 },
        };
        let union = Union { a: sphere1, b: sphere2 };
        
        // Test point between spheres
        let distance = union.sdf(Vec3::new(0.0, 0.0, 0.0));
        assert!(distance < 0.0); // Should be inside the union
    }

    #[test]
    fn test_aabb_union() {
        let sphere1 = Isometry {
            translation: Vec3::new(-1.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            primitive: Sphere { radius: 1.0 },
        };
        let sphere2 = Isometry {
            translation: Vec3::new(1.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            primitive: Sphere { radius: 1.0 },
        };
        let union = Union { a: sphere1, b: sphere2 };
        
        let aabb = union.aabb().unwrap();
        assert_eq!(aabb.min, Vec3A::new(-2.0, -1.0, -1.0));
        assert_eq!(aabb.max, Vec3A::new(2.0, 1.0, 1.0));
    }
}