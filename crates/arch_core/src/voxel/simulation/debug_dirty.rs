use bevy::prelude::*;

use crate::voxel::GRID_SCALE;
use crate::voxel::simulation::SimSettings;
use crate::voxel::simulation::data::{CHUNK_WIDTH, SimChunks, delinearize};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, display_cell_grids);
    app.add_systems(Update, display_dirty);
    // app.add_systems(Update, display_margolus_offset);
}

pub fn display_cell_grids(mut gizmos: Gizmos) {
    // basic cell grid
    // let cell_count = UVec2::new(16, 16);
    // let xz = Vec2::new(GRID_SCALE.x, GRID_SCALE.z);
    // let center = Vec3::new(xz.x / 2.0, 1.0, xz.y / 2.0)
    // Vec3::new(cell_count.x as f32, 1.0, cell_count.y as f32);
    //
    // gizmos.grid(
    // Isometry3d::new(center, Quat::from_rotation_x(90.0f32.to_radians())),
    // cell_count,
    // xz,
    // Color::srgba(0.0, 0.0, 0.5, 0.7),
    // );

    let cell_count = UVec2::new(4, 1);
    let xz = Vec2::new(GRID_SCALE.x, GRID_SCALE.z) * Vec2::new(4.0, 16.0);
    let center = Vec3::new(xz.x / 2.0, 1.0, xz.y / 2.0)
        * Vec3::new(cell_count.x as f32, 1.0, cell_count.y as f32);

    gizmos.grid(
        Isometry3d::new(center, Quat::from_rotation_x(90.0f32.to_radians())),
        cell_count,
        xz,
        Color::srgba(0.0, 0.0, 0.5, 0.7),
    );
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

pub fn display_margolus_offset(mut gizmos: Gizmos, chunks: Query<&SimChunks>) {
    for chunk in chunks {
        let offset = crate::voxel::simulation::data::MARGOLUS_OFFSETS[chunk.margolus_offset];

        let cell_count = UVec3::new(4, 4, 4);
        let chunk_scale = GRID_SCALE * Vec3::splat(CHUNK_WIDTH as f32);
        let block_offset = offset.as_vec3() * chunk_scale;

        let center = chunk_scale / 2.0 * cell_count.as_vec3();

        gizmos.grid_3d(
            Isometry3d::new(center + block_offset, Quat::IDENTITY),
            cell_count,
            chunk_scale * 2.0,
            Color::srgba(0.0, 0.5, 0.0, 0.7),
        );
    }
}
