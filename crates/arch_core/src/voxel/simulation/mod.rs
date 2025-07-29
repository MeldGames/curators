//! Voxel falling sands implementation.
//!
//! This needs to be relatively fast... going to be a
//! large experiment onto whether we can make this work or not.

use std::collections::BTreeSet;

use bevy::prelude::*;
#[cfg(feature = "trace")]
use tracing::*;

use crate::voxel::simulation::data::{SimChunks, CHUNK_LENGTH};
use crate::voxel::{Voxel, Voxels};

pub mod data;

pub fn plugin(app: &mut App) {
    app.register_type::<FallingSandTick>();
    app.insert_resource(FallingSandTick(0));
    app.add_systems(PostUpdate, falling_sands);
    app.add_systems(PostUpdate, update_render_voxels);
    // app.add_systems(Update, falling_sands);

    app.add_systems(Startup, || {
        info!("available parallelism: {:?}", std::thread::available_parallelism());
    });
    app.add_plugins(data::plugin);
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

// 62x1x62 chunks aligns with xz plane which has better contiguous memory ~12kb
// of memory for the surrounding 62x3x62 voxels what to do about boundaries?
// need at least somewhat uniform simulation, so spreading it out over multiple
// chunks would be good maybe offset spirals?:
//

// Take 2 on parallelization:
// - each thread has a work pool of a chunk, each chunk keeps a record of dirty
//   voxels (need to figure out a good way to store these too that isn't too
//   rough to set)
// - thread runs through each dirty voxel and marks where it'd like to go
// - collect all of the commands/movements, flatten duplicates based on distance
//   of the movement for determinism
// - give each thread a couple of dirty chunks to apply movements to and the
//   lists of movements
// 1 other thing is I really need to smooth this processing over multiple frames

// pub struct ChunkMovements {
//     pub movements: Vec<IVec3>,
// }

#[derive(Component, Clone)]
pub struct SimSwapBuffer(pub Vec<[u64; CHUNK_LENGTH / 64]>);

#[derive(Component, Clone)]
pub struct RenderSwapBuffer(pub Vec<[u64; CHUNK_LENGTH / 64]>);

pub fn update_render_voxels(mut grids: Query<(&mut Voxels, &mut RenderSwapBuffer)>) {
    for (mut grid, mut render_swap_buffer) in &mut grids {
        for (chunk_index, voxel_index) in grid.sim_chunks.render_updates(&mut render_swap_buffer.0)
        {
            let point =
                grid.sim_chunks.point_from_chunk_and_voxel_indices(chunk_index, voxel_index);
            let voxel = grid.sim_chunks.get_voxel_from_indices(chunk_index, voxel_index);
            grid.render_chunks.set_voxel(point, voxel);
        }
    }
}

pub fn falling_sands(
    mut grids: Query<(&mut Voxels, &mut SimSwapBuffer)>,
    mut sim_tick: ResMut<FallingSandTick>,
    mut ignore: Local<usize>,
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

    for (mut grid, mut sim_swap_buffer) in &mut grids {
        for (chunk_index, voxel_index) in grid.sim_chunks.sim_updates(&mut sim_swap_buffer.0) {
            #[cfg(feature = "trace")]
            let update_span = info_span!("update_voxel", iteration = counter);

            let sim_voxel = grid.sim_chunks.get_voxel_from_indices(chunk_index, voxel_index);

            // counter += 1;
            // if sim_voxel.is_simulated() {
            //     simulated_counter += 1;
            // } else {
            //     static_counter += 1;
            // };

            // TODO: delinearize indices into a point
            match sim_voxel {
                Voxel::Sand => {
                    // semi-solid
                    let point = grid
                        .sim_chunks
                        .point_from_chunk_and_voxel_indices(chunk_index, voxel_index);
                    simulate_semisolid(&mut grid.sim_chunks, point, sim_voxel, &sim_tick);
                },
                Voxel::Water { .. } | Voxel::Oil { .. } => {
                    // liquids
                    let point = grid
                        .sim_chunks
                        .point_from_chunk_and_voxel_indices(chunk_index, voxel_index);
                    simulate_liquid(&mut grid.sim_chunks, point, sim_voxel, &sim_tick);
                },
                // Voxel::Dirt => {
                //     let point = grid
                //         .sim_chunks
                //         .point_from_chunk_and_voxel_indices(chunk_index, voxel_index);
                    // simulate_structured(&mut grid.sim_chunks, point, sim_voxel, &sim_tick);
                // },
                _ => {}, // no-op
            }

            // if counter > MAX_UPDATE {
            //     break;
            // }
        }
    }

    // if simulated_counter > 0 {
    // info!("simulated {} voxels, static {} voxels", simulated_counter,
    // static_counter); }
}

const DOWN_DIAGONALS: [IVec3; 4] = [
    IVec3::NEG_Y.saturating_add(IVec3::NEG_X),
    IVec3::NEG_Y.saturating_add(IVec3::X),
    IVec3::NEG_Y.saturating_add(IVec3::NEG_Z),
    IVec3::NEG_Y.saturating_add(IVec3::Z),
];

const HORIZONTAL_ADJACENTS: [IVec3; 4] = [
    IVec3::NEG_X, // adjacent second
    IVec3::X,
    IVec3::NEG_Z,
    IVec3::Z,
];

const ALL_ADJACENTS: [IVec3; 6] =
    [IVec3::NEG_X, IVec3::X, IVec3::NEG_Z, IVec3::Z, IVec3::NEG_Y, IVec3::Y];

#[inline]
pub fn simulate_semisolid(
    grid: &mut SimChunks,
    point: IVec3,
    sim_voxel: Voxel,
    sim_tick: &FallingSandTick,
) {
    #[cfg(feature = "trace")]
    let simulate_semisolid_span = info_span!("simulate_semisolid");

    // let below_point = IVec3::from(point + IVec3::NEG_Y);
    // let below_voxel = grid.get_voxel(below_point);
    // if below_voxel.is_gas() || below_voxel.is_liquid() {
    //     grid.set_voxel(below_point, sim_voxel);
    //     grid.set_voxel(point, below_voxel);
    //     return;
    // }

    for &diagonal in [IVec3::NEG_Y].iter().chain(DOWN_DIAGONALS.iter()) {
        let voxel = grid.get_voxel(point + diagonal);
        if voxel.is_gas() || (diagonal == IVec3::NEG_Y && voxel.is_liquid()) {
            grid.set_voxel(point + diagonal, sim_voxel);
            grid.set_voxel(point, voxel);

            // grid.add_update_mask(chunk_index, voxel_index);
            // println!("simulated semisolid at {:?}", point + diagonal);
            return;
        }
    }
}

#[inline]
pub fn simulate_liquid(
    grid: &mut SimChunks,
    point: IVec3,
    sim_voxel: Voxel,
    sim_tick: &FallingSandTick,
) {
    #[cfg(feature = "trace")]
    let simulate_liquid_span = info_span!("simulate_liquid");

    // prioritize negative y
    let below_point = IVec3::from(point + IVec3::NEG_Y);
    let below_voxel = grid.get_voxel(below_point);
    if below_voxel.is_gas() || (below_voxel.is_liquid() && sim_voxel.denser(below_voxel)) {
        let new_sim_voxel = match sim_voxel {
            Voxel::Water { .. } => Voxel::Water { lateral_energy: 32 },
            Voxel::Oil { .. } => Voxel::Oil { lateral_energy: 32 },
            _ => sim_voxel,
        };

        grid.set_voxel(below_point, new_sim_voxel);
        grid.set_voxel(point, below_voxel);
        return;
    }

    for diagonal in DOWN_DIAGONALS.iter().cycle().skip((sim_tick.0 % 4) as usize).take(4) {
        let voxel = grid.get_voxel(IVec3::from(point + diagonal));
        if voxel.is_gas() {
            let new_sim_voxel = match sim_voxel {
                Voxel::Water { .. } => Voxel::Water { lateral_energy: 32 },
                Voxel::Oil { .. } => Voxel::Oil { lateral_energy: 32 },
                _ => sim_voxel,
            };

            grid.set_voxel(point + diagonal, new_sim_voxel);
            grid.set_voxel(point, voxel);
            return;
        }
    }

    for adjacent in HORIZONTAL_ADJACENTS.iter().cycle().skip((sim_tick.0 % 4) as usize).take(4) {
        let voxel = grid.get_voxel(IVec3::from(point + adjacent));
        if voxel.is_gas() {
            let new_sim_voxel = match sim_voxel {
                Voxel::Water { lateral_energy } => {
                    if lateral_energy == 0 {
                        // if !below_voxel.is_liquid() {
                        //     grid.set_voxel(point, Voxel::Air);
                        // }
                        // return;
                    }

                    Voxel::Water { lateral_energy: lateral_energy - 1 }
                },
                Voxel::Oil { lateral_energy } => {
                    if lateral_energy == 0 {
                        // if !below_voxel.is_liquid() {
                        //     grid.set_voxel(point, Voxel::Air);
                        // }
                        // return;
                    }
                    Voxel::Oil { lateral_energy: lateral_energy - 1 }
                },
                _ => sim_voxel,
            };

            grid.set_voxel(point + adjacent, new_sim_voxel);
            grid.set_voxel(point, voxel);
            return;
        }
    }
}

#[inline]
pub fn simulate_structured(
    grid: &mut SimChunks,
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

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::voxel::simulation::DOWN_DIAGONALS;

    fn neighbors(point: IVec3) -> Vec<IVec3> {
        let mut neighbors = Vec::new();
        for y in -1..=1 {
            for x in -1..=1 {
                for z in -1..=1 {
                    neighbors.push(point + IVec3::new(x, y, z));
                }
            }
        }

        neighbors
    }

    // #[test]
    // fn updates_for_swaps() {
    //     // merge 2 update masks for a point + a swap point, so we don't try to access
    //     // the bitmasks as much.
    //     let points = DOWN_DIAGONALS;

    //     for point in points {
    //         let mut offsets = neighbors(IVec3::ZERO).extend(neighbors(point).iter());
    //         offsets.dedup();
    //         offsets.sort_by(|&a, &b| a.y.cmp(&b.y).then(a.x.cmp(&b.x).then(a.z.cmp(&b.z))));
    //     }

    //     let name = "DIAGONALS";
    //     println!("const {}_UPDATE_OFFSETS: [IVec3; {}] = [", name, offsets.len());
    //     for offset in offsets {
    //         println!("    IVec3::new({}, {}, {})", offset.x, offset.y, offset.z);
    //     }

    //     println!("];");
    // }
}
