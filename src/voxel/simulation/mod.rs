//! Voxel falling sands implementation.
//!
//! This needs to be relatively fast... going to be a
//! large experiment onto whether we can make this work or not.

use crate::voxel::{Voxel, Voxels};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    // app.add_systems(FixedPreUpdate, falling_sands);
    app.add_systems(Update, falling_sands);
}

pub fn falling_sands(mut grids: Query<&mut Voxels>, mut updates: Local<Vec<IVec3>>) {
    const MAX_UPDATE: usize = 10_000;
    let mut counter = 0;

    for mut grid in &mut grids {
        updates.extend(grid.update_voxels.drain(..));
        updates.sort_by(|a, b| b.y.cmp(&a.y).then(b.x.cmp(&a.x)).then(b.z.cmp(&a.z)));

        while let Some(point) = updates.pop() {
            match grid.get_voxel(point) {
                Voxel::Sand => {
                    counter += 1;

                    const SWAP_POINTS: [[i32; 3]; 5] =
                        [[0, -1, 0], [1, -1, 0], [0, -1, 1], [-1, -1, 0], [0, -1, -1]];

                    for swap_point in SWAP_POINTS {
                        if let Voxel::Air =
                            grid.get_voxel(IVec3::from(point + IVec3::from(swap_point)))
                        {
                            grid.set_voxel(point + IVec3::from(swap_point), Voxel::Sand);
                            grid.set_voxel(point, Voxel::Air);
                            break;
                        }
                    }
                },
                _ => {}, // no-op
            }

            if counter > MAX_UPDATE {
                break;
            }
        }
    }
}
