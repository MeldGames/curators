use core::f32;

use bevy::prelude::*;

use super::voxel_grid::{Voxel, VoxelGrid, VoxelState};

pub struct VoxelPickPlugin;
impl Plugin for VoxelPickPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_cursor);
    }
}

pub fn draw_cursor(
    camera_query: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,

    mut grids: Query<(&GlobalTransform, &mut VoxelGrid)>,
    input: Res<ButtonInput<MouseButton>>,
    mut gizmos: Gizmos,
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
    let Ok((grid_transform, mut grid)) = grids.single_mut() else {
        return;
    };
    let hit = grid.cast_ray(grid_transform, ray, f32::INFINITY, Some(&mut gizmos));

    // Draw a circle just above the ground plane at that position.
    if let Some(hit) = hit {
        // let direct_point = ray.origin + hit.distance_to_grid * GRID_SCALE;

        // info!("hit: {:?}", hit);
        let point_ivec: IVec3 = hit.voxel.into();
        let point: Vec3 = point_ivec.as_vec3() + Vec3::splat(0.5);

        let normal_ivec: IVec3 = hit.normal.map(|n| n.into()).unwrap_or(IVec3::new(0, 1, 0));
        let normal: Vec3 = normal_ivec.as_vec3();

        let point_with_normal = point + normal * 0.501;
        let world_space_point = grid_transform.transform_point(point_with_normal);

        gizmos.circle(
            Isometry3d::new(world_space_point, Quat::from_rotation_arc(Vec3::Z, normal)),
            0.05,
            Color::WHITE,
        );

        // gizmos.circle(
        // Isometry3d::new(direct_point, Quat::from_rotation_arc(Vec3::Z, normal)),
        // 0.05,
        // Color::srgb(1.0, 0.0, 0.0),
        // );

        if input.just_pressed(MouseButton::Right) {
            // Place block
            let normal_block: [i32; 3] = (point_ivec + normal_ivec).into();
            if grid.in_bounds(normal_block.into()) {
                grid.set(normal_block, VoxelState::from(Voxel::Dirt));
            }
        } else if input.just_pressed(MouseButton::Left) {
            let scaled = IVec3::new(point_ivec.x / 5 * 5, point_ivec.y, point_ivec.z / 5 * 5);

            // Remove block
            for x in 0..5 {
                for z in 0..5 {
                    let offset = IVec3::new(x, 0, z);
                    let break_point = scaled + offset;
                    if grid.in_bounds(break_point.into()) {
                        if grid.voxel(break_point.into()).breakable() {
                            grid.set(break_point.into(), VoxelState::from(Voxel::Air));
                        }
                    }
                }
            }
        }
    }
}
