//! Voxel falling sands implementation.
//!
//! This needs to be relatively fast... going to be a
//! large experiment onto whether we can make this work or not.

use crate::voxel::{Voxel, Voxels};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(FixedPreUpdate, falling_sands);
}

pub fn falling_sands(mut grids: Query<&mut Voxels>, mut points: Local<Vec<IVec3>>) {
    for mut grid in &mut grids {
        for point in grid.update_voxels.drain(..) {
            points.push(point);
        }

        for point in points.drain(..) {
            match grid.get_voxel(point) {
                Voxel::Sand => {
                    const SWAP_POINTS: [IVec3; 5] = [
                        ivec3(0, -1, 0),
                        ivec3(1, -1, 0),
                        ivec3(0, -1, 1),
                        ivec3(-1, -1, 0),
                        ivec3(0, -1, -1),
                    ];
                    for swap_point in SWAP_POINTS {
                        if let Voxel::Air = grid.get_voxel(point + swap_point) {
                            grid.set_voxel(point + swap_point, Voxel::Sand);
                            grid.set_voxel(point, Voxel::Air);
                            break;
                        }
                    }
                },
                _ => {}, // no-op
            }
        }
    }
}
