//! Create a list of chunk points that are generally in the frustum of the active camera.

use bevy::platform::collections::HashMap;
use bevy::render::primitives::Aabb;
use bevy::{prelude::*, render::primitives::Frustum};
use bevy_math::Affine3A;

use crate::voxel::Voxels;

pub fn plugin(app: &mut App) {
    app.register_type::<FrustumChunks>();
    app.insert_resource(FrustumChunks::default());

    app.add_systems(First, FrustumChunks::intersecting_chunks);
}

#[derive(Resource, Clone, Default, Reflect, Debug, Deref, DerefMut)]
#[reflect(Resource)]
pub struct FrustumChunks(HashMap<(Entity, IVec3), FrustumChunk>); // (voxel_entity, chunk_point) -> distance_to_camera

#[derive(Copy, Clone, Default, Reflect, Debug, PartialEq)]
pub struct FrustumChunk {
    pub distance_to_camera: f32,
    pub distance_to_camera_ray: f32,
}

impl FrustumChunk {
    pub fn heuristic(&self) -> f32 {
        const DIST_TO_CAMERA_WEIGHT: f32 = 0.5;
        const DIST_TO_CAMERA_RAY_WEIGHT: f32 = 0.3;

        self.distance_to_camera * DIST_TO_CAMERA_WEIGHT
            + self.distance_to_camera_ray * DIST_TO_CAMERA_RAY_WEIGHT
    }
}

impl PartialOrd for FrustumChunk {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;
        let a = self.heuristic();
        let b = other.heuristic();
        Some(if a > b {
            Ordering::Less
        } else if a < b {
            Ordering::Greater
        } else {
            Ordering::Equal
        })
    }
}

impl FrustumChunks {
    pub fn intersecting_chunks(
        mut frustum_chunks: ResMut<FrustumChunks>,
        voxels: Query<(Entity, &GlobalTransform, &Voxels)>,
        cameras: Query<(&GlobalTransform, &Camera, &Frustum)>,
        mut gizmos: Gizmos,
    ) {
        let Some((camera_transform, camera, frustum)) =
            cameras.iter().find(|(_, camera, _)| camera.is_active)
        else {
            return;
        };

        let Ok((voxel_entity, voxel_transform, voxels)) = voxels.single() else {
            return;
        };

        frustum_chunks.clear();

        use crate::voxel::GRID_SCALE;
        use crate::voxel::mesh::unpadded::SIZE as CHUNK_SIZE;
        let chunk_size = Vec3::splat(CHUNK_SIZE as f32) * Vec3::from(GRID_SCALE);

        for chunk_pos in voxels.render_chunks.chunk_pos_iter() {
            let min = chunk_pos.as_vec3() * chunk_size;
            let max = min + chunk_size;
            let aabb = Aabb::from_min_max(min, max);
            // let intersects = frustum.contains_aabb(&aabb, &camera_transform.affine().inverse());
            let intersects = frustum.intersects_obb(&aabb, &Affine3A::IDENTITY, true, true);
            let color = if intersects {
                let chunk_worldspace = voxel_transform.transform_point(Vec3::from(aabb.center));
                let distance_to_camera = camera_transform.translation().distance(chunk_worldspace);

                let camera_origin = camera_transform.translation();
                let camera_ray = camera_transform.forward();
                let relative_chunk = chunk_worldspace - camera_origin;
                let along_ray = relative_chunk.dot(*camera_ray);
                let closest_point = camera_origin + camera_ray * along_ray;
                let distance_to_camera_ray = closest_point.distance(chunk_worldspace);

                let chunk = FrustumChunk {
                    distance_to_camera,
                    distance_to_camera_ray,
                };

                frustum_chunks.insert((voxel_entity, chunk_pos), chunk);
                Color::srgb(0.0, 1.0, 0.0)
            } else {
                Color::srgb(1.0, 0.0, 0.0)
            };

            // gizmos.line(Vec3::new(min.x, min.y, min.z), Vec3::new(max.x, min.y, min.z), color);
            // gizmos.line(Vec3::new(min.x, min.y, min.z), Vec3::new(min.x, max.y, min.z), color);
            // gizmos.line(Vec3::new(min.x, min.y, min.z), Vec3::new(min.x, min.y, max.z), color);
        }

        // info!("intersecting: {:?}", frustum_chunks);
    }
}
