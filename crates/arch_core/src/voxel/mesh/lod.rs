use bevy::prelude::*;

use crate::voxel::mesh::binary_greedy::{Chunks, GreedyMeshes, GridChunk};
use crate::voxel::mesh::surface_net::SurfaceNetMeshes;
use crate::voxel::mesh::{ChangedChunk, SurfaceNet};

pub fn plugin(app: &mut App) {
    app.register_type::<Lod>().register_type::<LodSettings>();

    app.insert_resource(LodSettings { ..default() });

    app.add_systems(PreUpdate, pick_lod);
    app.add_observer(mesh_method_changed::<SurfaceNet>);
}

#[derive(Component, Debug, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct Lod(pub usize);

#[derive(Resource, Debug, Reflect)]
#[reflect(Resource)]
pub struct LodSettings {
    pub threshold_2: f32,
    // pub threshold_3: 150.0,
}

impl Default for LodSettings {
    fn default() -> Self {
        Self { threshold_2: 150.0 }
    }
}

pub fn pick_lod(
    cameras: Query<(&GlobalTransform, &Camera), Changed<GlobalTransform>>,
    mut last_pos: Local<Vec3>,

    mut chunks: Query<(&GlobalTransform, &mut Lod)>,
    lod_settings: Res<LodSettings>,
) {
    let Some((camera_transform, camera)) = cameras.iter().find(|(_, c)| c.is_active) else {
        return;
    };

    if last_pos.distance(camera_transform.translation()) < 1.0 {
        return;
    }

    *last_pos = camera_transform.translation();

    let threshold_2_squared = lod_settings.threshold_2 * lod_settings.threshold_2;
    for (chunk_transform, mut lod) in chunks {
        let distance =
            chunk_transform.translation().distance_squared(camera_transform.translation());

        let new_lod = if distance < threshold_2_squared { 1 } else { 2 };

        if lod.0 != new_lod {
            lod.0 = new_lod;
        }
    }
}

pub fn mesh_method_changed<M: Component>(
    trigger: Trigger<OnAdd, M>,
    mut writer: EventWriter<ChangedChunk>,
    grid_chunk: Query<&GridChunk>,
) {
    let Ok(grid_chunk) = grid_chunk.get(trigger.target()) else {
        return;
    };

    writer.write(ChangedChunk { grid_entity: grid_chunk.entity, chunk_point: grid_chunk.position });
}

// pub fn mesh_method(
//     mut commands: Commands,
//     chunks: Query<(Entity, &Lod, &SurfaceNetMeshes, &GreedyMeshes),
// Changed<Lod>>,     mut visibility: Query<&mut Visibility>,
// ) {
//     for (chunk_entity, lod, surface_net_meshes, greedy_meshes) in chunks {
//         if lod.0 == 1 {
//
// commands.entity(chunk_entity).insert(SurfaceNet).remove::<BinaryGreedy>();

//             for (_, entity) in surface_net_meshes.iter() {
//                 let Ok(mut vis) = visibility.get_mut(*entity) else { continue
// };                 *vis = Visibility::Inherited;
//             }

//             for (_, entity) in greedy_meshes.iter() {
//                 let Ok(mut vis) = visibility.get_mut(*entity) else { continue
// };                 *vis = Visibility::Hidden;
//             }
//         } else {
//
// commands.entity(chunk_entity).insert(BinaryGreedy).remove::<SurfaceNet>();

//             for (_, entity) in surface_net_meshes.iter() {
//                 let Ok(mut vis) = visibility.get_mut(*entity) else { continue
// };                 *vis = Visibility::Hidden;
//             }

//             for (_, entity) in greedy_meshes.iter() {
//                 let Ok(mut vis) = visibility.get_mut(*entity) else { continue
// };                 *vis = Visibility::Inherited;
//             }
//         }
//     }
// }
