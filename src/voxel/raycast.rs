//! Taken from https://gitlab.com/athilenius/fast-voxel-traversal-rs
//!
//! Just adapted to work directly with our types
use bevy::prelude::*;

use crate::voxel::voxel_aabb::VoxelAabb;

impl VoxelAabb {
    pub fn traverse_ray(&self, ray: Ray3d, length: f32) -> VoxelRayIterator {
        VoxelRayIterator::new(self.clone(), ray, length)
    }

    #[inline(always)]
    pub(crate) fn contains_point(&self, point: IVec3) -> bool {
        point.cmpge(self.min).all() && point.cmplt(self.max).all()
    }
}

#[derive(Debug, Clone, Copy, Reflect)]
pub struct VoxelHit {
    pub chunk: IVec3,
    pub voxel: IVec3,
    pub world_space: Vec3,

    pub distance: f32,
    pub normal: Option<IVec3>,
}

#[derive(Debug, Clone, Copy, Reflect)]
pub struct Hit {
    pub distance: f32,
    pub voxel: IVec3,
    pub normal: Option<IVec3>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct VoxelRayIterator {
    /// Bounding volume we are checking against. Each 1 unit is a voxel.
    volume: VoxelAabb,
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
impl VoxelRayIterator {
    pub fn new(volume: VoxelAabb, ray: Ray3d, ray_length: f32) -> Self {
        let mut position = ray.origin;

        // Normalize direction vector
        let direction = Vec3::from(ray.direction).normalize();

        // How long we have traveled thus far (modified by initial 'jump to volume
        // bounds').
        let t = 0.0;

        const BACKOFF: f32 = 1.0;

        // If the point it outside the chunk, AABB entry/exit test to 'jump ahead'.
        let Some((entrypoint, exitpoint)) =
            aabb_intersections(volume, position, direction, ray_length)
        else {
            // Chunk AABB test failed, no way we could intersect a voxel.
            return Self { done: true, ..Default::default() };
        };

        // Back the hit off at least 1 voxel
        position = entrypoint - direction * BACKOFF;
        let to_volume = (ray.origin - entrypoint).length();

        // Max distance we can travel. This is either the ray length, or the current `t`
        // plus the corner to corner length of the voxel volume.
        // TODO: Improve this:
        // - this could ideally be the length from the current position to the exit
        //   point of the aabb.
        let max_distance = f32::min(ray_length, position.distance(exitpoint));

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

impl Iterator for VoxelRayIterator {
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

fn aabb_intersections(
    volume: VoxelAabb,
    from: Vec3,
    direction: Vec3,
    distance: f32,
) -> Option<(Vec3, Vec3)> {
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

    // Early exit: no intersection, or outside max distance
    if t_min > t_max || t_max < 0.0 || t_min > distance {
        return None;
    }

    // Clamp to ray's valid distance range
    let entry = from + direction * t_min.clamp(0.0, distance);
    let exit = from + direction * t_max.clamp(0.0, distance);

    Some((entry, exit))
}

pub fn plugin(mut app: &mut App) {
    app.register_type::<DebugRaycast>();
    app.insert_resource(DebugRaycast { show_chunks: false, show_voxels: false, debug_ray: None });
    app.add_systems(PreUpdate, debug_raycast);
}

#[derive(Resource, Debug, Reflect)]
#[reflect(Resource)]
pub struct DebugRaycast {
    /// Show chunk boundaries in raycast.
    pub show_chunks: bool,
    /// Show voxel boundaries in raycast.
    pub show_voxels: bool,

    pub debug_ray: Option<Ray3d>,
}

impl DebugRaycast {
    fn enabled(&self) -> bool {
        self.show_chunks || self.show_voxels
    }
}

pub fn debug_raycast(
    camera_query: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,

    mut voxels: Query<(&GlobalTransform, &crate::voxel::Voxels)>,

    mut gizmos: Gizmos,

    input: Res<ButtonInput<MouseButton>>,

    mut debug_raycast: ResMut<DebugRaycast>,
) {
    if !debug_raycast.enabled() {
        return;
    }

    let Some((camera, camera_transform)) = camera_query.iter().find(|(camera, _)| camera.is_active)
    else {
        return;
    };
    let Some(window) = windows.iter().find(|window| window.focused) else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // Calculate a ray pointing from the camera into the world based on the cursor's
    // position.
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    if input.just_pressed(MouseButton::Middle) {
        debug_raycast.debug_ray = Some(ray);
    }

    // https://github.com/cgyurgyik/fast-voxel-traversal-algorithm/blob/master/overview/FastVoxelTraversalOverview.md

    // Calculate if and where the ray is hitting a voxel.
    let Ok((voxels_transform, mut voxels)) = voxels.single_mut() else {
        info!("No voxels found");
        return;
    };

    let test_ray = if let Some(last_ray) = debug_raycast.debug_ray { last_ray } else { ray };

    const GREEN: Color = Color::srgb(0.0, 1.0, 0.0);
    const RED: Color = Color::srgb(1.0, 0.0, 0.0);
    const BLUE: Color = Color::srgb(0.0, 0.0, 1.0);
    for hit in voxels.ray_iter(voxels_transform, test_ray, 1_000.0) {
        use crate::voxel::GRID_SCALE;
        const CHUNK_SIZE: Vec3 = Vec3::splat(crate::voxel::chunk::unpadded::SIZE as f32);

        // info!("- hit: {:?}", hit);

        // Generate chunk aabbs that we sampled
        if debug_raycast.show_chunks {
            #[allow(non_snake_case)]
            let SCALED_CHUNK_SIZE: Vec3 = CHUNK_SIZE * GRID_SCALE;

            let pos = hit.chunk.as_vec3();
            gizmos.cuboid(
                Transform {
                    translation: pos * SCALED_CHUNK_SIZE + SCALED_CHUNK_SIZE / 2.0,
                    scale: SCALED_CHUNK_SIZE,
                    ..default()
                },
                Color::srgb(1.0, 0.0, 0.0),
            );
        }

        // Generate voxel aabbs that we sampled
        if debug_raycast.show_voxels {
            let pos = hit.voxel.as_vec3();
            gizmos.cuboid(
                Transform {
                    translation: pos * GRID_SCALE + GRID_SCALE / 2.0,
                    scale: GRID_SCALE,
                    ..default()
                },
                Color::srgb(1.0, 0.0, 0.0),
            );
        }

        if let Some(voxel) = voxels.get_voxel(hit.voxel) {
            if voxel.pickable() {
                break;
            }
        }
    }
}
