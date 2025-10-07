use bevy::prelude::*;

use crate::voxel::simulation::SimSettings;
use crate::voxel::simulation::data::{CHUNK_WIDTH, SimChunks, delinearize};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, display_dirty);
}

pub fn display_dirty(sims: Query<(&SimChunks,)>, mut gizmos: Gizmos, settings: Res<SimSettings>) {
    if !settings.display_flagged && !settings.display_modified {
        return;
    }

    for (chunks,) in sims {
        for (chunk_point, (chunk_key, dirty_key)) in chunks.from_chunk_point.iter() {
            let chunk = chunks.chunks.get(*chunk_key).unwrap();
            let dirty = chunks.dirty.get(*dirty_key).unwrap();

            if settings.display_flagged {
                for voxel_index in dirty.iter() {
                    let chunk_voxel_start = chunk_point.0 * IVec3::splat(CHUNK_WIDTH as i32);
                    let relative_voxel_point = delinearize(voxel_index);
                    let voxel_point = chunk_voxel_start + relative_voxel_point;
                    gizmos.cuboid(
                        Transform {
                            scale: crate::voxel::GRID_SCALE,
                            translation: voxel_point.as_vec3() * crate::voxel::GRID_SCALE
                                + crate::voxel::GRID_SCALE / 2.0,
                            ..default()
                        },
                        Color::srgba(1.0, 0.0, 0.0, 0.5),
                    );
                }
            }

            if settings.display_modified {
                for voxel_index in chunk.modified.iter() {
                    let chunk_voxel_start = chunk_point.0 * IVec3::splat(CHUNK_WIDTH as i32);
                    let relative_voxel_point = delinearize(voxel_index);
                    let voxel_point = chunk_voxel_start + relative_voxel_point;
                    gizmos.cuboid(
                        Transform {
                            scale: crate::voxel::GRID_SCALE,
                            translation: voxel_point.as_vec3() * crate::voxel::GRID_SCALE
                                + crate::voxel::GRID_SCALE / 2.0,
                            ..default()
                        },
                        Color::srgba(0.0, 1.0, 0.0, 0.5),
                    );
                }
            }
        }
    }
}
