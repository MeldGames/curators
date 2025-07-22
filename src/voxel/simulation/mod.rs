//! Voxel falling sands implementation.
//!
//! This needs to be relatively fast... going to be a
//! large experiment onto whether we can make this work or not.

use std::collections::BTreeSet;

use crate::voxel::{Voxel, Voxels, unpadded, voxels::VoxelUpdate};
use bevy::prelude::*;

#[cfg(feature = "trace")]
use tracing::*;

pub fn plugin(app: &mut App) {
    app.register_type::<FallingSandTick>();
    app.insert_resource(FallingSandTick(0));
    app.add_systems(FixedPreUpdate, falling_sands);
    // app.add_systems(Update, falling_sands);
}

// Make islands of voxels fall if unsupported.
pub fn islands(mut grids: Query<&mut Voxels>) {}

#[derive(Resource, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[reflect(Resource)]
pub struct FallingSandTick(pub u32);

// 62x1x62 vertical chunks separated by 2 vertical slices,
// need buffers around boundaries to avoid race conditions

// requirements:
// - deterministic
// - no overlaps with other work groups for parallelism
// - no more than ~6000 voxels simulated per work group

// 62x1x62 chunks aligns with xz plane which has better contiguous memory ~12kb of memory for the surrounding 62x3x62 voxels
// what to do about boundaries?
// need at least somewhat uniform simulation, so spreading it out over multiple chunks would be good
// maybe offset spirals?:
//

// Take 2 on parallelization:
// - each thread has a work pool of a chunk, each chunk keeps a record of dirty voxels (need to figure out a good way to store these too that isn't too rough to set)
// - thread runs through each dirty voxel and marks where it'd like to go
// - collect all of the commands/movements, flatten duplicates based on distance of the movement for determinism
// - give each thread a couple of dirty chunks to apply movements to and the lists of movements
// 1 other thing is I really need to smooth this processing over multiple frames

// pub struct ChunkMovements {
//     pub movements: Vec<IVec3>,
// }

pub struct VoxelMovement {
    pub from: IVec3,
    pub from_chunk: IVec3,
    pub to: IVec3,
}

pub fn falling_sands(
    mut grids: Query<&mut Voxels>,
    mut sim_tick: ResMut<FallingSandTick>,
    mut ignore: Local<usize>,
    mut updates: Local<Vec<VoxelUpdate>>,
) {
    *ignore = (*ignore + 1) % 4; // 60 / 4 ticks per second
    if *ignore != 0 {
        // return;
    }

    #[cfg(feature = "trace")]
    let falling_sands_span = info_span!("falling_sands");

    sim_tick.0 = (sim_tick.0 + 1) % (u32::MAX / 2);

    // const MAX_UPDATE: usize = 1_000_000;
    let mut counter = 0;
    let mut simulated_counter = 0;
    let mut static_counter = 0;

    for mut grid in &mut grids {
        {
            // #[cfg(feature = "trace")]
            // let update_management_span = info_span!("update_management");

            updates.clear();
            std::mem::swap(&mut *updates, &mut grid.update_voxels);
            match sim_tick.0 % 4 {
                0 => {
                    updates.sort_by(|a, b| a.y.cmp(&b.y).then(b.z.cmp(&a.z)).then(b.x.cmp(&a.x)));
                },
                1 => {
                    updates.sort_by(|a, b| a.y.cmp(&b.y).then(b.x.cmp(&a.x)).then(b.z.cmp(&a.z)));
                },
                2 => {
                    updates.sort_by(|a, b| a.y.cmp(&b.y).then(a.x.cmp(&b.x)).then(a.z.cmp(&b.z)));
                },
                3 => {
                    updates.sort_by(|a, b| a.y.cmp(&b.y).then(a.z.cmp(&b.z)).then(a.x.cmp(&b.x)));
                },
                _ => unreachable!(),
            }

            updates.dedup();
        }

        for point in updates.iter().map(|p| p.0) {
            #[cfg(feature = "trace")]
            let update_span = info_span!("update_voxel", iteration = counter);

            let sim_voxel = grid.get_voxel(point);
            // counter += 1;
            // if sim_voxel.is_simulated() {
            //     simulated_counter += 1;
            // } else {
            //     static_counter += 1;
            // };

            match sim_voxel {
                Voxel::Sand => {
                    // semi-solid
                    simulate_semisolid(&mut grid, point, sim_voxel, &sim_tick);
                },
                Voxel::Water { .. } | Voxel::Oil { .. } => {
                    // liquids
                    simulate_liquid(&mut grid, point, sim_voxel, &sim_tick);
                },
                Voxel::Dirt => {
                    simulate_structured(&mut grid, point, sim_voxel, &sim_tick);
                },
                _ => {}, // no-op
            }

            // if counter > MAX_UPDATE {
            //     break;
            // }
        }
    }

    // if simulated_counter > 0 {
    // info!("simulated {} voxels, static {} voxels", simulated_counter, static_counter);
    // }
}

#[inline]
pub fn simulate_semisolid(
    grid: &mut Voxels,
    point: IVec3,
    sim_voxel: Voxel,
    sim_tick: &FallingSandTick,
) {
    #[cfg(feature = "trace")]
    let simulate_semisolid_span = info_span!("simulate_semisolid");

    const SWAP_POINTS: [IVec3; 5] =
        [IVec3::NEG_Y, ivec3(1, -1, 0), ivec3(0, -1, 1), ivec3(-1, -1, 0), ivec3(0, -1, -1)];

    for swap_point in SWAP_POINTS {
        let voxel = grid.get_voxel(point + swap_point);
        if voxel.is_liquid() || voxel.is_gas() {
            grid.set_voxel(point + swap_point, Voxel::Sand);
            grid.set_voxel(point, voxel);
            break;
        }
    }
}

#[inline]
pub fn simulate_liquid(
    grid: &mut Voxels,
    point: IVec3,
    sim_voxel: Voxel,
    sim_tick: &FallingSandTick,
) {
    #[cfg(feature = "trace")]
    let simulate_liquid_span = info_span!("simulate_liquid");

    let swap_criteria =
        |voxel: Voxel| voxel.is_gas() || (voxel.is_liquid() && sim_voxel.denser(voxel));

    const SWAP_POINTS: [IVec3; 8] = [
        IVec3::NEG_Y.saturating_add(IVec3::NEG_X), // diagonals first
        IVec3::NEG_Y.saturating_add(IVec3::X),
        IVec3::NEG_Y.saturating_add(IVec3::NEG_Z),
        IVec3::NEG_Y.saturating_add(IVec3::Z),
        IVec3::NEG_X, // adjacent second
        IVec3::X,
        IVec3::NEG_Z,
        IVec3::Z,
    ];

    // prioritize negative y
    let below_point = IVec3::from(point + IVec3::NEG_Y);
    let below_voxel = grid.get_voxel(below_point);
    if swap_criteria(below_voxel) {
        grid.set_voxel(below_point, sim_voxel);
        grid.set_voxel(point, below_voxel);
    } else {
        // for swap_point in SWAP_POINTS.iter().cycle().skip((sim_tick.0 % 4) as usize).take(8) {
        for swap_point in SWAP_POINTS.iter().cycle().skip((sim_tick.0 % 8) as usize).take(8) {
            let voxel = grid.get_voxel(IVec3::from(point + swap_point));
            if swap_criteria(voxel) {
                grid.set_voxel(point + swap_point, sim_voxel);
                grid.set_voxel(point, voxel);
                break;
            }
        }
    }
}

#[inline]
pub fn simulate_structured(
    grid: &mut Voxels,
    point: IVec3,
    sim_voxel: Voxel,
    sim_tick: &FallingSandTick,
) {
    #[cfg(feature = "trace")]
    let simulate_structured_span = info_span!("simulate_structured");

    let below_voxel = grid.get_voxel(point + IVec3::new(0, -1, 0));
    if below_voxel == Voxel::Air {
        const SURROUNDING: [IVec3; 5] =
            [ivec3(-1, 0, 0), ivec3(1, 0, 0), ivec3(0, 0, -1), ivec3(0, 0, 1), ivec3(0, 1, 0)];

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
}
