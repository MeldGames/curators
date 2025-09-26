use crate::voxel::simulation::{
    data::{CHUNK_WIDTH, SimChunks, delinearize},
    set::ChunkSet,
};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.register_type::<DebugDirtyCells>();
    app.insert_resource(DebugDirtyCells { flagged: true, modified: false });
    app.add_systems(Update, display_dirty);
}

#[derive(Resource, Debug, Reflect)]
pub struct DebugDirtyCells {
    pub flagged: bool,
    pub modified: bool,
}

pub fn display_dirty(sims: Query<(&SimChunks,)>, mut gizmos: Gizmos) {
    for (chunks,) in sims {
        for (chunk_point, (chunk_key, dirty_key)) in chunks.from_chunk_point.iter() {
            // let chunk = chunks.chunks.get(*chunk_key).unwrap();
            let dirty = chunks.dirty.get(*dirty_key).unwrap();

            for voxel_index in dirty.iter() {
                let chunk_voxel_start = chunk_point.0 * IVec3::splat(CHUNK_WIDTH as i32);
                let relative_voxel_point = delinearize(voxel_index);
                let voxel_point = chunk_voxel_start + relative_voxel_point;
                gizmos.cuboid(
                    Transform {
                        scale: crate::voxel::GRID_SCALE,
                        translation: voxel_point.as_vec3() * crate::voxel::GRID_SCALE
                            - crate::voxel::GRID_SCALE / 2.0,
                        ..default()
                    },
                    Color::srgba(0.0, 1.0, 0.0, 1.0),
                );
            }
        }
    }
}
