use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use crate::voxel::raycast::{Hit, VoxelHit};
use crate::voxel::{Voxel, Voxels};

pub struct VoxelPickPlugin;
impl Plugin for VoxelPickPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CursorVoxel(None));
        app.add_systems(First, cursor_voxel);
        app.add_systems(Update, draw_cursor);
    }
}

#[derive(Resource, Debug, Clone, Deref)]
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
    let Ok((voxels_transform, mut voxels)) = voxels.single_mut() else {
        info!("No voxels found");
        return;
    };

    let hit = voxels.cast_ray(voxels_transform, ray, 1_000.0);
    cursor_voxel.0 = hit;
}

pub fn draw_cursor(
    cursor_voxel: Res<CursorVoxel>,
    mut voxels: Query<(&GlobalTransform, &mut Voxels)>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    key_input: Res<ButtonInput<KeyCode>>,
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

        // gizmos.circle(
        // Isometry3d::new(direct_point, Quat::from_rotation_arc(Vec3::Z,
        // normal)), 0.05,
        // Color::srgb(1.0, 0.0, 0.0),
        // );

        if mouse_input.just_pressed(MouseButton::Right)
            || (mouse_input.pressed(MouseButton::Right) && key_input.pressed(KeyCode::ShiftLeft))
        {
            // Place block
            let normal_block: [i32; 3] = (point_ivec + normal_ivec).into();
            voxels.set_voxel(normal_block.into(), Voxel::Dirt);
        } else if mouse_input.just_pressed(MouseButton::Left)
            || (mouse_input.pressed(MouseButton::Left) && key_input.pressed(KeyCode::ShiftLeft))
        {
            // Remove block
            let break_point = point_ivec;
            if let Some(voxel) = voxels.get_voxel(break_point.into()) {
                if voxel.breakable() {
                    voxels.set_voxel(break_point.into(), Voxel::Air);
                }
            }
        }
    }
}
