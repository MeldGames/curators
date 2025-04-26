//! Basic static box colliders

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::voxel::voxel_grid::{Voxel, VoxelGrid};

pub struct VoxelBoxColliderPlugin;
impl Plugin for VoxelBoxColliderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default());
        app.add_plugins(PhysicsDebugPlugin::default());

        app.add_systems(Update, spawn_box_colliders.before(VoxelGrid::clear_changed_system));
        // app.add_systems(Update, spawn_ball);
    }
}

pub fn spawn_mesh_collider() {}

pub fn spawn_box_colliders(
    mut commands: Commands,
    grids: Query<(Entity, &GlobalTransform, &VoxelGrid), Changed<VoxelGrid>>,
) {
    for (entity, global_transform, grid) in &grids {
        let mut colliders: Vec<(Vec3, Quat, Collider)> = Vec::new();
        for point in grid.point_iter() {
            let point_ivec3: IVec3 = point.into();
            if !grid.in_bounds(point) {
                continue;
            }

            if !grid.voxel(point).pickable() {
                continue;
            }

            let collider_point = point_ivec3.as_vec3() + Vec3::splat(0.5);
            colliders.push((collider_point, Quat::IDENTITY, Collider::cuboid(1.0, 1.0, 1.0)));
        }

        let mut collider = Collider::compound(colliders);
        collider.set_scale(crate::voxel::GRID_SCALE, 32);
        commands.entity(entity).insert((collider, RigidBody::Static));
    }
}

pub fn spawn_ball(mut commands: Commands, input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::KeyB) {
        commands.spawn((
            RigidBody::Dynamic,
            Transform::from_xyz(1.0, 10.0, 1.0),
            Collider::sphere(0.5),
        ));
    }
}
