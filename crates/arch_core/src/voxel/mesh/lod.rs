use bevy::prelude::*;

use crate::voxel::mesh::{BinaryGreedy, SurfaceNet};

pub fn plugin(app: &mut App) {
    app.register_type::<Lod>();

    app.add_systems(PreUpdate, (pick_lod, mesh_method).chain());
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

pub fn mesh_method(mut commands: Commands, chunks: Query<(Entity, &Lod)>) {
    for (entity, lod) in chunks {
        if lod.0 == 1 {
            commands.entity(entity).insert(SurfaceNet).remove::<BinaryGreedy>();
        } else {
            commands.entity(entity).insert(BinaryGreedy).remove::<SurfaceNet>();
        }
    }
}
