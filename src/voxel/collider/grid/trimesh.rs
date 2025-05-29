//! Static trimesh collider

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::voxel::{UpdateVoxelMeshSet, Voxel, VoxelChunk};

pub struct VoxelTrimeshColliderPlugin;
impl Plugin for VoxelTrimeshColliderPlugin {
    fn build(&self, app: &mut App) {
        // app.add_systems(PostUpdate,
        // spawn_mesh_collider.after(UpdateVoxelMeshSet));
        // app.add_systems(Update, spawn_ball);
    }
}

#[derive(Component)]
pub struct VoxelTrimeshCollider;

pub fn spawn_mesh_collider(
    mut commands: Commands,
    grids: Query<(Entity, &GlobalTransform, &VoxelChunk, &Children)>,
    voxel_mesh: Query<&Mesh3d>,
    collider_child: Query<Entity, With<VoxelTrimeshCollider>>,
    mut colliders: Query<&mut Collider>,
    meshes: Res<Assets<Mesh>>,
) {
    for (entity, global_transform, grid, children) in &grids {
        let Some(mesh) = children.iter().find_map(|child| voxel_mesh.get(child).ok()) else {
            continue;
        };

        let Some(mesh) = meshes.get(mesh) else {
            warn!("no mesh found in assets");
            continue;
        };

        let flags = TrimeshFlags::MERGE_DUPLICATE_VERTICES
            | TrimeshFlags::FIX_INTERNAL_EDGES
            | TrimeshFlags::DELETE_DEGENERATE_TRIANGLES
            | TrimeshFlags::DELETE_DUPLICATE_TRIANGLES;

        let Some(mut new_collider) = Collider::trimesh_from_mesh_with_config(mesh, flags) else {
            info!("cannot create trimesh from mesh");
            continue;
        };
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
                    Transform::from_translation(Vec3::splat(0.0)),
                    VoxelTrimeshCollider,
                ))
                .id();
            commands.entity(entity).add_child(collider_child);
        }
    }
}
