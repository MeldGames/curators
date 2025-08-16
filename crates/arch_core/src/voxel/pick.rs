use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

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
    mouse_input: Res<ButtonInput<MouseButton>>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut gizmos: Gizmos,

    mut brush_index: Local<usize>,
    mut voxel_index: Local<usize>,
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

        let brushes: Vec<&dyn sdf::Sdf> = vec![
            &sdf::Torus { minor_radius: 2.0, major_radius: 5.0 },
            &sdf::Sphere { radius: 4.0 },
            // &sdf::Sphere { radius: 0.2 },
        ];

        let brush_voxels: Vec<Voxel> =
            vec![Voxel::Dirt, Voxel::Sand, Voxel::Water { lateral_energy: 4 }, Voxel::Oil {
                lateral_energy: 4,
            }];

        if key_input.just_pressed(KeyCode::KeyB) {
            *brush_index = (*brush_index + 1) % brushes.len();
        }

        if key_input.just_pressed(KeyCode::KeyV) {
            *voxel_index = (*voxel_index + 1) % brush_voxels.len();
        }

        let brush = brushes[*brush_index];
        let brush_voxel = brush_voxels[*voxel_index];

        if mouse_input.just_pressed(MouseButton::Right)
            || (mouse_input.pressed(MouseButton::Right) && key_input.pressed(KeyCode::ShiftLeft))
        {
            // Place block
            let normal_block = point_ivec + normal_ivec;

            for raster_voxel in crate::sdf::voxel_rasterize::rasterize(brush, RasterConfig {
                clip_bounds: Aabb3d { min: Vec3A::splat(-1000.0), max: Vec3A::splat(1000.0) },
                grid_scale: crate::voxel::GRID_SCALE,
                pad_bounds: Vec3::splat(3.0),
            }) {
                let point = normal_block + raster_voxel.point;
                if raster_voxel.distance < 0.0 {
                    if voxels.get_voxel(point) == Voxel::Air && point.y > -10 {
                        voxels.set_voxel(point, brush_voxel);
                    }
                }
            }

            // voxels.set_voxel(normal_block.into(), Voxel::Sand);
        } else if mouse_input.just_pressed(MouseButton::Left)
            || (mouse_input.pressed(MouseButton::Left) && key_input.pressed(KeyCode::ShiftLeft))
        {
            // Remove block
            let break_point = point_ivec;

            for raster_voxel in crate::sdf::voxel_rasterize::rasterize(brush, RasterConfig {
                clip_bounds: Aabb3d { min: Vec3A::splat(-1000.0), max: Vec3A::splat(1000.0) },
                grid_scale: crate::voxel::GRID_SCALE,
                pad_bounds: Vec3::splat(3.0),
            }) {
                let point = break_point + raster_voxel.point;
                if raster_voxel.distance < 0.0 {
                    if voxels.get_voxel(point).breakable() && point.y > -10 {
                        voxels.set_voxel(point, Voxel::Air);
                    }
                }
            }
        } else if key_input.pressed(KeyCode::KeyU) {
            for raster_voxel in crate::sdf::voxel_rasterize::rasterize(brush, RasterConfig {
                clip_bounds: Aabb3d { min: Vec3A::splat(-1000.0), max: Vec3A::splat(1000.0) },
                grid_scale: crate::voxel::GRID_SCALE,
                pad_bounds: Vec3::splat(3.0),
            }) {
                let point = point_ivec + raster_voxel.point;
                if raster_voxel.distance < 0.0 {
                    voxels.sim_chunks.push_sim_update(point);
                }
            }
        }
    }
}
