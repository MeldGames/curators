use core::f32;

use bevy::prelude::*;

use crate::voxel::{Voxel, Voxels};

pub struct VoxelPickPlugin;
impl Plugin for VoxelPickPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_cursor);
    }
}

pub fn draw_cursor(
    camera_query: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,

    mut chunks: Query<(&GlobalTransform, &mut Voxels)>,
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
    let Ok((chunk_transform, mut chunk)) = chunks.single_mut() else {
        return;
    };
    let hit = chunk.cast_ray(chunk_transform, ray, f32::INFINITY, Some(&mut gizmos));

    // Draw a circle just above the ground plane at that position.
    if let Some(hit) = hit {
        // let direct_point = ray.origin + hit.distance_to_chunk * chunk_SCALE;

        // info!("hit: {:?}", hit);
        let point_ivec: IVec3 = hit.voxel.into();
        let point: Vec3 = point_ivec.as_vec3() + Vec3::splat(0.5);

        let normal_ivec: IVec3 = hit.normal.map(|n| n.into()).unwrap_or(IVec3::new(0, 1, 0));
        let normal: Vec3 = normal_ivec.as_vec3();

        let point_with_normal = point + normal * 0.501;
        let world_space_point = chunk_transform.transform_point(point_with_normal);

        gizmos.circle(
            Isometry3d::new(world_space_point, Quat::from_rotation_arc(Vec3::Z, normal)),
            0.05,
            Color::WHITE,
        );

        // gizmos.circle(
        // Isometry3d::new(direct_point, Quat::from_rotation_arc(Vec3::Z,
        // normal)), 0.05,
        // Color::srgb(1.0, 0.0, 0.0),
        // );

        // if input.just_pressed(MouseButton::Right) {
        //     // Place block
        //     let normal_block: [i32; 3] = (point_ivec + normal_ivec).into();
        //     if chunk.in_chunk_bounds(normal_block.into()) {
        //         chunk.set(normal_block, Voxel::Dirt);
        //     }
        // } else if input.just_pressed(MouseButton::Left) {
        //     // Remove block
        //     let break_point = point_ivec;
        //     if chunk.in_chunk_bounds(break_point.into()) {
        //         if chunk.voxel(break_point.into()).breakable() {
        //             chunk.set(break_point.into(), Voxel::Air);
        //         }
        //     }
        // }
    }
}
