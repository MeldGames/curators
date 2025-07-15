//! Voxel falling sands implementation.
//!
//! This needs to be relatively fast... going to be a
//! large experiment onto whether we can make this work or not.

use crate::voxel::{Voxel, Voxels};
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(FixedPreUpdate, falling_sands);
    // app.add_systems(Update, falling_sands);
}

// Make islands of voxels fall if unsupported.
pub fn islands(mut grids: Query<&mut Voxels>) {}

pub fn falling_sands(mut grids: Query<&mut Voxels>, mut updates: Local<Vec<IVec3>>) {
    // const MAX_UPDATE: usize = 1_000_000;
    // let mut counter = 0;

    for mut grid in &mut grids {
        updates.extend(grid.update_voxels.drain(..));
        updates.sort_by(|a, b| b.y.cmp(&a.y).then(b.x.cmp(&a.x)).then(b.z.cmp(&a.z)));
        updates.dedup();

        while let Some(point) = updates.pop() {
            let sim_voxel = grid.get_voxel(point);
            match sim_voxel {
                Voxel::Sand => { // semi-solid
                    // counter += 1;

                    const SWAP_POINTS: [[i32; 3]; 5] =
                        [[0, -1, 0], [1, -1, 0], [0, -1, 1], [-1, -1, 0], [0, -1, -1]];

                    for swap_point in SWAP_POINTS {
                        let voxel = grid.get_voxel(IVec3::from(point + IVec3::from(swap_point)));
                        if voxel.is_liquid() || voxel.is_gas() {
                            grid.set_voxel(point + IVec3::from(swap_point), Voxel::Sand);
                            grid.set_voxel(point, voxel);
                            break;
                        }
                    }
                },
                Voxel::Water | Voxel::Oil => { // liquids
                    // counter += 1;

                    const SWAP_POINTS: [IVec3; 5] =[
                        IVec3::NEG_Y,
                        IVec3::NEG_X,
                        IVec3::X,
                        IVec3::NEG_Z,
                        IVec3::Z,
                    ];

                    for swap_point in SWAP_POINTS {
                        let voxel = grid.get_voxel(IVec3::from(point + IVec3::from(swap_point)));
                        if voxel.is_gas() || (voxel.is_liquid() && sim_voxel.denser(voxel)) {
                            grid.set_voxel(point + IVec3::from(swap_point), sim_voxel);
                            grid.set_voxel(point, voxel);
                            break;
                        }
                    }
                },
                Voxel::Dirt => {
                    // counter += 1;

                    let below_voxel = grid.get_voxel(point + IVec3::new(0, -1, 0));
                    if below_voxel == Voxel::Air {
                        const SURROUNDING: [IVec3; 5] = [
                            ivec3(-1, 0, 0),
                            ivec3(1, 0, 0),
                            ivec3(0, 0, -1),
                            ivec3(0, 0, 1),
                            ivec3(0, 1, 0),
                        ];

                        let mut structured = false;
                        for check in SURROUNDING {
                            let check_voxel = grid.get_voxel(point + check);

                            if !check_voxel.is_liquid() && !check_voxel.is_gas() {
                                structured = true;
                                break;
                            }
                        }

                        if structured {
                            grid.set_voxel(point + IVec3::new(0, -1, 0), Voxel::Dirt);
                            grid.set_voxel(point, below_voxel);
                        }
                    }
                },
                _ => {}, // no-op
            }

            // if counter > MAX_UPDATE {
            //     break;
            // }
        }
    }
}
