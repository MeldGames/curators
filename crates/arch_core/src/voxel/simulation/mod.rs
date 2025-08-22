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

/// Hold new updates for this chunk on the stack instead of heap.
/// 
/// Only set the directly affected voxels in this mask, then later we will
/// spread the bits to the adjacent axes.
/// Z access can be spread via a simple << | and >> |:
/// `mask | (mask << 1) | (mask >> 1)`
/// 
/// X access can be spread via:
/// `mask | (mask << 16) | (mask >> 16)`
/// 
/// Y access can be spread via | to the masks +-4 ((16 * 16) / 64)
/// `mask[i - 4] | mask[i] | mask[i + 4]`
pub struct StackUpdates {
    center: [u64; 64],

    neg_z: [u64; 64],
    pos_z: [u64; 64],

    neg_x: [u64; 4],
    pos_x: [u64; 4],

    neg_y: [u64; 4],
    pos_y: [u64; 4],
}

impl StackUpdates {
    pub fn new() -> Self {
        Self {
            center: [0u64; 64],
            neg_z: [0u64; 64],
            pos_z: [0u64; 64],

            neg_x: [0u64; 4],
            pos_x: [0u64; 4],

            neg_y: [0u64; 4],
            pos_y: [0u64; 4],
        }
    }
}

pub fn falling_sands(
    mut grids: Query<(Entity, &mut Voxels, &mut SimSwapBuffer)>,
    mut sim_tick: ResMut<FallingSandTick>,

    sim_settings: Res<SimSettings>,
    mut changed_chunk_event: EventWriter<ChangedChunk>,
    // mut gizmos: Option<Gizmos>,
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
            // if sim_voxel.is_simulated() {
                let point = SimChunks::point_from_chunk_and_voxel_indices(chunk_point, voxel_index);
                sim_voxel.simulate(&mut grid.sim_chunks, point, &sim_tick);

                // if let Some(gizmos) = gizmos.as_mut() && sim_settings.display_simulated {
                //     gizmos.cuboid(
                //         Transform {
                //             translation: point.as_vec3() * GRID_SCALE,
                //             scale: GRID_SCALE,
                //             ..default()
                //         },
                //         Color::srgb(1.0, 0.0, 0.0),
                //     );
                // }
            // } else {
                // if let Some(gizmos) = gizmos.as_mut() && sim_settings.display_checked {
                //     let point =
                //         SimChunks::point_from_chunk_and_voxel_indices(chunk_point, voxel_index);
                //     gizmos.cuboid(
                //         Transform {
                //             translation: point.as_vec3() * GRID_SCALE,
                //             scale: GRID_SCALE,
                //             ..default()
                //         },
                //         Color::srgb(0.0, 0.0, 1.0),
                //     );
                // }
            // }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::voxel::simulation::StackUpdates;

    #[test]
    pub fn create_stack_updates() {
        let updates = StackUpdates::new();
    }
}