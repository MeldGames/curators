//! Static trimesh collider

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::voxel::voxel_grid::{Voxel, VoxelGrid};

pub struct VoxelTrimeshColliderPlugin;
impl Plugin for VoxelTrimeshColliderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_mesh_collider.before(VoxelGrid::clear_changed_system));
        // app.add_systems(Update, spawn_ball);
    }
}

#[derive(Component)]
pub struct VoxelTrimeshCollider;

pub fn spawn_mesh_collider(
    mut commands: Commands,
    grids: Query<(Entity, &GlobalTransform, &VoxelGrid, &Children), Changed<VoxelGrid>>,
    voxel_mesh: Query<&Mesh3d>,
    collider_child: Query<Entity, With<VoxelTrimeshCollider>>,
    mut colliders: Query<&mut Collider>,
    meshes: Res<Assets<Mesh>>,
) {
    for (entity, global_transform, grid, children) in &grids {
        let Some(mesh) = children.iter().find_map(|child| voxel_mesh.get(child).ok()) else {
            continue;
        };

        let mesh = meshes.get(mesh).unwrap();
        let mut new_collider = Collider::trimesh_from_mesh(mesh).unwrap();
        new_collider.set_scale(crate::voxel::GRID_SCALE, 32);

        if let Some(child) = children.iter().find_map(|child| collider_child.get(child).ok()) {
            let Ok(mut collider) = colliders.get_mut(child) else {
                continue;
            };
            *collider = new_collider;
        } else {
            let collider_child = commands
                .spawn((
                    Name::new("Collider"),
                    new_collider,
                    RigidBody::Static,
                    CollisionMargin(0.05),
                    Transform::from_translation(Vec3::splat(0.5)),
                    VoxelTrimeshCollider,
                ))
                .id();
            commands.entity(entity).add_child(collider_child);
        }
    }
}
