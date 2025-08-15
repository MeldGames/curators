use bevy::prelude::*;

use crate::voxel::mesh::binary_greedy::{Chunks, GreedyMeshes, GridChunk};
use crate::voxel::mesh::surface_net::SurfaceNetMeshes;
use crate::voxel::mesh::{BinaryGreedy, ChangedChunks, SurfaceNet};

pub fn plugin(app: &mut App) {
    app.register_type::<Lod>();

    app.add_systems(PreUpdate, (pick_lod, mesh_method).chain());
    app.add_observer(mesh_method_changed::<SurfaceNet>)
        .add_observer(mesh_method_changed::<BinaryGreedy>);
}

#[derive(Component, Debug, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct Lod(pub usize);

pub fn pick_lod(
    cameras: Query<(&GlobalTransform, &Camera)>,
    mut chunks: Query<(&GlobalTransform, &mut Lod)>,
) {
    let Some((camera_transform, camera)) = cameras.iter().find(|(_, c)| c.is_active) else {
        return;
    };

    for (chunk_transform, mut lod) in chunks {
        let threshold = 75.0;
        let threshold_squared = threshold * threshold;
        let distance =
            chunk_transform.translation().distance_squared(camera_transform.translation());

        let new_lod = if distance < threshold_squared { 1 } else { 2 };

        if lod.0 != new_lod {
            lod.0 = new_lod;
        }
    }
}

pub fn mesh_method_changed<M: Component>(
    trigger: Trigger<OnAdd, M>,
    mut writer: EventWriter<ChangedChunks>,
    grid_chunk: Query<&GridChunk>,
) {
    let Ok(grid_chunk) = grid_chunk.get(trigger.target()) else {
        return;
    };

    writer.write(ChangedChunks {
        voxel_entity: grid_chunk.entity,
        changed_chunks: vec![grid_chunk.position],
    });
}

pub fn mesh_method(
    mut commands: Commands,
    chunks: Query<(Entity, &Lod, &SurfaceNetMeshes, &GreedyMeshes)>,
    mut visibility: Query<&mut Visibility>,
) {
    for (chunk_entity, lod, surface_net_meshes, greedy_meshes) in chunks {
        if lod.0 == 1 {
            commands.entity(chunk_entity).insert(SurfaceNet).remove::<BinaryGreedy>();

            for (_, entity) in surface_net_meshes.iter() {
                let Ok(mut vis) = visibility.get_mut(*entity) else { continue };
                *vis = Visibility::Inherited;
            }

            for (_, entity) in greedy_meshes.iter() {
                let Ok(mut vis) = visibility.get_mut(*entity) else { continue };
                *vis = Visibility::Hidden;
            }
        } else {
            commands.entity(chunk_entity).insert(BinaryGreedy).remove::<SurfaceNet>();

            for (_, entity) in surface_net_meshes.iter() {
                let Ok(mut vis) = visibility.get_mut(*entity) else { continue };
                *vis = Visibility::Hidden;
            }

            for (_, entity) in greedy_meshes.iter() {
                let Ok(mut vis) = visibility.get_mut(*entity) else { continue };
                *vis = Visibility::Inherited;
            }
        }
    }
}
