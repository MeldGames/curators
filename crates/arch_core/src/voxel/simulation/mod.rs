//! Voxel falling sands implementation.
//!
//! This needs to be relatively fast... going to be a
//! large experiment onto whether we can make this work or not.

use bevy::prelude::*;
#[cfg(feature = "trace")]
use tracing::*;

use crate::voxel::mesh::ChangedChunk;
use crate::voxel::simulation::data::{CHUNK_LENGTH, SimChunks, UpdateBuffer};
use crate::voxel::{GRID_SCALE, Voxel, Voxels};

pub mod data;
pub mod kinds;
pub mod morton;
pub mod rle;

pub fn plugin(app: &mut App) {
    app.register_type::<FallingSandTick>().register_type::<SimSettings>();

    app.insert_resource(FallingSandTick(0));
    app.insert_resource(SimSettings::default());

    app.add_systems(FixedPostUpdate, falling_sands);
    app.add_systems(PostUpdate, sim_settings);

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
pub struct SimSwapBuffer(pub UpdateBuffer);

#[derive(Resource, Copy, Clone, Reflect)]
#[reflect(Resource)]
pub struct SimSettings {
    /// Run the simulation.
    pub run: bool,

    /// Display voxels that are being actively simulated.
    pub display_simulated: bool,

    /// Display voxels marked for updates but not simulated.
    pub display_checked: bool,
}

impl Default for SimSettings {
    fn default() -> Self {
        Self { run: true, display_simulated: false, display_checked: false }
    }
}

pub fn sim_settings(mut sim_settings: ResMut<SimSettings>, input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::KeyL) {
        sim_settings.display_simulated = sim_settings.display_simulated;
    }
}

pub fn falling_sands(
    mut grids: Query<(Entity, &mut Voxels, &mut SimSwapBuffer)>,
    mut sim_tick: ResMut<FallingSandTick>,
    mut changed_chunk_event: EventWriter<ChangedChunk>,
    mut chunk_points: Local<Vec<IVec3>>,
    mut gizmos: Gizmos,
    sim_settings: Res<SimSettings>,
) {
    if !sim_settings.run {
        return;
    }

    #[cfg(feature = "trace")]
    let falling_sands_span = info_span!("falling_sands").entered();

    sim_tick.0 = (sim_tick.0 + 1) % (u32::MAX / 2);

    for (grid_entity, mut grid, mut sim_swap_buffer) in &mut grids {
        sim_swap_buffer.0.clear();

        for (chunk_point, voxel_index) in grid.sim_chunks.sim_updates(&mut sim_swap_buffer.0) {
            #[cfg(feature = "trace")]
            let update_span = info_span!("update_voxel").entered();

            changed_chunk_event.write(ChangedChunk { grid_entity, chunk_point });

            let sim_voxel = grid.sim_chunks.get_voxel_from_indices(chunk_point, voxel_index);
            if sim_voxel.is_simulated() {
                let point = SimChunks::point_from_chunk_and_voxel_indices(chunk_point, voxel_index);
                sim_voxel.simulate(&mut grid.sim_chunks, point, &sim_tick);

                if sim_settings.display_simulated {
                    gizmos.cuboid(
                        Transform {
                            translation: point.as_vec3() * GRID_SCALE,
                            scale: GRID_SCALE,
                            ..default()
                        },
                        Color::srgb(1.0, 0.0, 0.0),
                    );
                }
            } else {
                if sim_settings.display_checked {
                    let point =
                        SimChunks::point_from_chunk_and_voxel_indices(chunk_point, voxel_index);
                    gizmos.cuboid(
                        Transform {
                            translation: point.as_vec3() * GRID_SCALE,
                            scale: GRID_SCALE,
                            ..default()
                        },
                        Color::srgb(0.0, 0.0, 1.0),
                    );
                }
            }
        }
    }
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

// #[inline]
// pub fn simulate_structured(
//     grid: &mut SimChunks,
//     point: IVec3,
//     sim_voxel: Voxel,
//     sim_tick: &FallingSandTick,
// ) {
//     #[cfg(feature = "trace")]
//     let simulate_structured_span =
// info_span!("simulate_structured").entered();

//     let below_voxel = grid.get_voxel(point + IVec3::new(0, -1, 0));
//     if below_voxel == Voxel::Air {
//         const SURROUNDING: [IVec3; 5] =
//             [ivec3(-1, 0, 0), ivec3(1, 0, 0), ivec3(0, 0, -1), ivec3(0, 0,
// 1), ivec3(0, 1, 0)];

//         let mut structured = false;
//         for check in SURROUNDING {
//             let check_voxel = grid.get_voxel(point + check);

//             if !check_voxel.is_liquid() && !check_voxel.is_gas() {
//                 structured = true;
//                 break;
//             }
//         }

//         if structured {
//             grid.set_voxel(point + IVec3::new(0, -1, 0), Voxel::Dirt);
//             grid.set_voxel(point, below_voxel);
//         }
//     }
// }

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
    //     // merge 2 update masks for a point + a swap point, so we don't try
    // to access     // the bitmasks as much.
    //     let points = DOWN_DIAGONALS;

    //     for point in points {
    //         let mut offsets =
    // neighbors(IVec3::ZERO).extend(neighbors(point).iter());
    //         offsets.dedup();
    //         offsets.sort_by(|&a, &b|
    // a.y.cmp(&b.y).then(a.x.cmp(&b.x).then(a.z.cmp(&b.z))));     }

    //     let name = "DIAGONALS";
    //     println!("const {}_UPDATE_OFFSETS: [IVec3; {}] = [", name,
    // offsets.len());     for offset in offsets {
    //         println!("    IVec3::new({}, {}, {})", offset.x, offset.y,
    // offset.z);     }

    //     println!("];");
    // }
}
