use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;
use bevy_enhanced_input::prelude::*;

use crate::sdf;
use crate::sdf::voxel_rasterize::RasterConfig;
use crate::voxel::raycast::VoxelHit;
use crate::voxel::{Voxel, Voxels};

pub struct VoxelPickPlugin;
impl Plugin for VoxelPickPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CursorVoxel>().register_type::<VoxelHit>();

        app.insert_resource(CursorVoxel(None));

        app.add_systems(First, cursor_voxel);
        app.add_systems(Update, draw_cursor);
    }
}

#[derive(Resource, Debug, Clone, Deref, Reflect)]
#[reflect(Resource)]
pub struct CursorVoxel(Option<VoxelHit>);

impl CursorVoxel {
    pub fn hit(&self) -> &Option<VoxelHit> {
        &self.0
    }
}

pub fn cursor_voxel(
    camera_query: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,

    mut voxels: Query<(&GlobalTransform, &Voxels)>,

    mut cursor_voxel: ResMut<CursorVoxel>,
) {
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

    // https://github.com/cgyurgyik/fast-voxel-traversal-algorithm/blob/master/overview/FastVoxelTraversalOverview.md

    // Calculate if and where the ray is hitting a voxel.
    let Ok((voxels_transform, voxels)) = voxels.single_mut() else {
        info!("No voxels found");
        return;
    };

    let hit = voxels.cast_ray(voxels_transform, ray, 1_000.0);
    cursor_voxel.0 = hit;
}

pub fn draw_cursor(
    cursor_voxel: Res<CursorVoxel>,
    mut voxels: Query<(&GlobalTransform, &mut Voxels)>,
    mut gizmos: Gizmos,
) {
    // Calculate if and where the ray is hitting a voxel.
    let Ok((voxel_transform, mut voxels)) = voxels.single_mut() else {
        info!("No voxels found");
        return;
    };

    // Draw a circle just above the ground plane at that position.
    if let Some(hit) = cursor_voxel.hit() {
        // let direct_point = ray.origin + hit.distance_to_chunk * chunk_SCALE;

        // info!("hit: {:?}", hit);
        let point_ivec: IVec3 = hit.voxel.into();
        let point: Vec3 = point_ivec.as_vec3() + Vec3::splat(0.5);

        let normal_ivec: IVec3 = hit.normal.map(|n| n.into()).unwrap_or(IVec3::new(0, 1, 0));
        let normal: Vec3 = normal_ivec.as_vec3();

        let point_with_normal = point + normal * 0.501;
        let world_space_point = voxel_transform.transform_point(point_with_normal);

        gizmos.circle(
            Isometry3d::new(world_space_point, Quat::from_rotation_arc(Vec3::Z, normal)),
            0.05,
            Color::WHITE,
        );
    }
}
