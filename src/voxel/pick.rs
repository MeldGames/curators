use bevy::prelude::*;

use super::voxel_grid::VoxelGrid;

pub struct VoxelPickPlugin;
impl Plugin for VoxelPickPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_cursor);
    }
}

pub fn draw_cursor(
    camera_query: Single<(&Camera, &GlobalTransform)>,
    grids: Query<&VoxelGrid>,
    windows: Single<&Window>,
    mut gizmos: Gizmos,
) {
    let (camera, camera_transform) = *camera_query;

    let Some(cursor_position) = windows.cursor_position() else {
        return;
    };

    // Calculate a ray pointing from the camera into the world based on the cursor's position.

    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    // https://github.com/cgyurgyik/fast-voxel-traversal-algorithm/blob/master/overview/FastVoxelTraversalOverview.md

    // Calculate if and where the ray is hitting a voxel.
    let grid = grids.single();
    let hit = grid.cast_local_ray(ray);

    // Draw a circle just above the ground plane at that position.

    if let Some(hit) = hit {
        //info!("hit: {:?}", hit);
        let point: IVec3 = hit.voxel.into();
        let point: Vec3 = point.as_vec3() + Vec3::splat(0.5);

        let normal: IVec3 = hit.normal.map(|n| n.into()).unwrap_or(IVec3::new(0, 1, 0));
        let normal: Vec3 = normal.as_vec3();

        gizmos.circle(
            Isometry3d::new(point + normal * 0.51, Quat::from_rotation_arc(Vec3::Z, normal)),
            0.2,
            Color::WHITE,
        );
    }
}
