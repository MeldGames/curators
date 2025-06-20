//! Taken from https://gitlab.com/athilenius/fast-voxel-traversal-rs
//!
//! Just adapted to work directly with our types
use bevy::prelude::*;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BoundingVolume3 {
    pub min: IVec3,
    pub max: IVec3,
}

impl BoundingVolume3 {
    pub fn traverse_ray(&self, ray: Ray3d, length: f32) -> VoxelRay3Iterator {
        VoxelRay3Iterator::new(self.clone(), ray, length)
    }

    #[inline(always)]
    pub(crate) fn contains_point(&self, point: IVec3) -> bool {
        point.cmpge(self.min).all() && point.cmplt(self.max).all()
    }

    pub fn size(&self) -> IVec3 {
        self.max - self.min + IVec3::ONE
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VoxelHit {
    pub chunk: IVec3,
    pub voxel: IVec3,
    pub world_space: Vec3,

    pub distance: f32,
    pub normal: Option<IVec3>,
}

#[derive(Debug, Clone, Copy)]
pub struct Hit {
    pub distance: f32,
    pub voxel: IVec3,
    pub normal: Option<IVec3>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct VoxelRay3Iterator {
    /// Bounding volume we are checking against. Each 1 unit is a voxel.
    volume: BoundingVolume3,
    /// Maximum distance we can travel.
    max_distance: f32,
    /// Voxel we are currently at.
    current_voxel: IVec3,
    step: IVec3,
    delta: Vec3,

    /// Distance we have travelled on each axis + 1 step
    t_max: Vec3,
    /// Distance travelled from entrypoint to current position/voxel.
    t: f32,
    /// Distance travelled from origin to entrypoint.
    to_volume: f32,
    normal: Option<IVec3>,
    done: bool,
}

// Based on https://github.com/fenomas/fast-voxel-raycast/blob/master/index.js
impl VoxelRay3Iterator {
    pub fn new(volume: BoundingVolume3, ray: Ray3d, ray_length: f32) -> Self {
        let mut position = ray.origin;

        // Normalize direction vector
        let direction = Vec3::from(ray.direction).normalize();
        info!("dir: {:?}", direction);

        // How long we have traveled thus far (modified by initial 'jump to volume
        // bounds').
        let mut t = 0.0;

        // If the point it outside the chunk, AABB test to 'jump ahead'.
        let to_volume = if !volume.contains_point(position.floor().as_ivec3()) {
            // First AABB test the chunk bounds
            let entrypoint = test_aabb_of_chunk(volume, position, direction, ray_length);

            // Chunk AABB test failed, no way we could intersect a voxel.
            if let Some(entrypoint) = entrypoint {
                // Back the hit off at least 1 voxel
                position = entrypoint - direction * 2.0;

                (ray.origin - entrypoint).length()
            } else {
                return Self { done: true, ..Default::default() };
            }
        } else {
            0.0 // we are already inside
        };

        // Max distance we can travel. This is either the ray length, or the current `t`
        // plus the corner to corner length of the voxel volume.
        // let max_distance = f32::min(ray_length, t + volume.size().as_vec3().length()
        // + 2.0);
        let max_distance = ray_length;

        // The starting voxel for the raycast.
        let voxel = position.floor().as_ivec3();

        // The direction we are stepping for each component.
        let step = direction.signum().as_ivec3();

        // Just abs(Vec3::ONE / d) but accounts for zeros in the distance vector.
        let delta = (Vec3::new(
            if direction.x.abs() < f32::EPSILON { f32::INFINITY } else { 1.0 / direction.x },
            if direction.y.abs() < f32::EPSILON { f32::INFINITY } else { 1.0 / direction.y },
            if direction.z.abs() < f32::EPSILON { f32::INFINITY } else { 1.0 / direction.z },
        ))
        .abs();

        let dist = Vec3::new(
            if step.x > 0 {
                voxel.x as f32 + 1.0 - position.x
            } else {
                position.x - voxel.x as f32
            },
            if step.y > 0 {
                voxel.y as f32 + 1.0 - position.y
            } else {
                position.y - voxel.y as f32
            },
            if step.z > 0 {
                voxel.z as f32 + 1.0 - position.z
            } else {
                position.z - voxel.z as f32
            },
        );

        // The nearest voxel boundary.
        let t_max = Vec3::new(
            if delta.x < f32::INFINITY { delta.x * dist.x } else { f32::INFINITY },
            if delta.y < f32::INFINITY { delta.y * dist.y } else { f32::INFINITY },
            if delta.z < f32::INFINITY { delta.z * dist.z } else { f32::INFINITY },
        );

        Self {
            volume,
            max_distance,
            current_voxel: voxel,
            step,
            delta,
            t_max,
            t,
            to_volume,
            normal: None,
            done: false,
        }
    }
}

impl Iterator for VoxelRay3Iterator {
    type Item = Hit;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        while self.t <= self.max_distance {
            // Test if the current traverse is within the volume.
            let mut hit = None;
            if self.volume.contains_point(self.current_voxel) {
                hit = Some(Hit {
                    distance: self.t + self.to_volume,
                    voxel: self.current_voxel.into(),
                    normal: self.normal.map(|n| n.into()),
                });
            }

            // Select the smallest t_max
            if self.t_max.x < self.t_max.y {
                if self.t_max.x < self.t_max.z {
                    self.current_voxel.x += self.step.x;
                    self.t = self.t_max.x;
                    self.t_max.x += self.delta.x;
                    self.normal = Some(IVec3::new(-self.step.x, 0, 0));
                } else {
                    self.current_voxel.z += self.step.z;
                    self.t = self.t_max.z;
                    self.t_max.z += self.delta.z;
                    self.normal = Some(IVec3::new(0, 0, -self.step.z));
                }
            } else {
                if self.t_max.y < self.t_max.z {
                    self.current_voxel.y += self.step.y;
                    self.t = self.t_max.y;
                    self.t_max.y += self.delta.y;
                    self.normal = Some(IVec3::new(0, -self.step.y, 0));
                } else {
                    self.current_voxel.z += self.step.z;
                    self.t = self.t_max.z;
                    self.t_max.z += self.delta.z;
                    self.normal = Some(IVec3::new(0, 0, -self.step.z));
                }
            }

            if hit.is_some() {
                return hit;
            }
        }

        self.done = true;
        return None;
    }
}

fn test_aabb_of_chunk(
    volume: BoundingVolume3,
    from: Vec3,
    direction: Vec3,
    distance: f32,
) -> Option<Vec3> {
    let min = volume.min.as_vec3();
    let max = volume.max.as_vec3();

    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::INFINITY;

    for i in 0..3 {
        if direction[i].abs() < f32::EPSILON {
            // Ray is parallel to the slab. Reject if origin not within slab.
            if from[i] < min[i] || from[i] > max[i] {
                return None;
            }
        } else {
            let inv_d = 1.0 / direction[i];
            let mut t0 = (min[i] - from[i]) * inv_d;
            let mut t1 = (max[i] - from[i]) * inv_d;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            t_min = t_min.max(t0);
            t_max = t_max.min(t1);
        }
    }

    if t_min <= t_max && t_max >= 0.0 && t_min <= distance {
        Some(from + direction * t_min.max(0.0))
    } else {
        None
    }
}
