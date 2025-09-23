//! Voxel falling sands implementation.
//!
//! This needs to be relatively fast... going to be a
//! large experiment onto whether we can make this work or not.

use bevy::prelude::*;
#[cfg(feature = "trace")]
use tracing::*;

use crate::voxel::simulation::data::{delinearize, ChunkPoint, SimChunk, SimChunks, CHUNK_LENGTH};
use crate::voxel::simulation::set::ChunkSet;
use crate::voxel::tree::VoxelNode;
use crate::voxel::{GRID_SCALE, Voxel, Voxels};

pub mod data;
pub mod gpu;
pub mod kinds;
pub mod morton;
pub mod rle;
pub mod view;
pub mod set;

pub fn plugin(app: &mut App) {
    app.register_type::<FallingSandTick>().register_type::<SimSettings>();

    app.insert_resource(FallingSandTick(0));
    app.insert_resource(SimSettings::default());

    app.add_systems(FixedPostUpdate, (add_sand, pull_from_tree, falling_sands, propagate_to_tree).chain());
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

#[derive(Resource, Copy, Clone, Reflect)]
#[reflect(Resource)]
pub struct SimSettings {
    /// Run the simulation.
    pub run: bool,

    /// Display voxels that are being actively simulated.
    pub display_simulated: bool,

    /// Display voxels marked for updates but not simulated.
    pub display_checked: bool,

    /// How many threads for the simulation.
    pub sim_threads: usize,
}

impl Default for SimSettings {
    fn default() -> Self {
        let threads =
            std::thread::available_parallelism().map(|nonzero| nonzero.get()).unwrap_or(4);
        Self { run: true, display_simulated: false, display_checked: false, sim_threads: threads }
    }
}

pub fn sim_settings(mut sim_settings: ResMut<SimSettings>, input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::KeyL) {
        sim_settings.display_simulated = sim_settings.display_simulated;
    }
}

// Pull relevant chunks from the 64tree into our linear array.
pub fn pull_from_tree(mut grids: Query<(Entity, &Voxels, &mut SimChunks)>) {
    for (_grid_entity, voxels, mut sim_chunks) in &mut grids {
        for z in 0..16{
            for x in 0..16 {
                for y in 0..4 {
                    let chunk_point = IVec3::new(x, y, z);
                    let voxels = match voxels.tree.root.get_chunk(chunk_point) {
                        VoxelNode::Solid { voxel, .. } => {
                            Some([*voxel; CHUNK_LENGTH])
                        }
                        VoxelNode::Leaf { leaf } => {
                            Some(**leaf)
                        }
                        _ => None
                    };

                    if let Some(voxels) = voxels {
                        sim_chunks.add_chunk(ChunkPoint(chunk_point), voxels);
                    }
                }
            }
        }
    } 
}

pub fn propagate_to_tree(mut grids: Query<(Entity, &mut Voxels, &SimChunks)>) {
    for (_grid_entity, mut voxels, sim_chunks) in &mut grids {
        for (chunk_point, (chunk_key, _dirty_key)) in &sim_chunks.from_chunk_point {
            // info!("propagating to tree: {:?}", chunk_point);
            let sim_chunk = sim_chunks.chunks.get(*chunk_key).unwrap();

            if !sim_chunk.modified.any_set() {
                continue;
            }

            match voxels.tree.get_chunk_mut(**chunk_point) {
                VoxelNode::Solid { .. } => {
                    voxels.tree.set_chunk_data(**chunk_point, sim_chunk.voxels);
                }
                VoxelNode::Leaf { leaf } => {
                    for voxel_index in sim_chunk.modified.iter() {
                        leaf[voxel_index] = sim_chunk.voxels[voxel_index];
                    }
                }
                _ => {},
            }
        }
    }
}

pub fn add_sand(mut grids: Query<(Entity, &mut Voxels, &SimChunks)>) {
    for (_grid_entity, mut voxels, sim_chunks) in &mut grids {
        voxels.set_voxel(IVec3::new(10, 20, 10), Voxel::Sand);
    }
}

pub fn falling_sands(
    mut grids: Query<(Entity, &mut SimChunks)>,
    mut sim_tick: ResMut<FallingSandTick>,

    sim_settings: Res<SimSettings>,
    // mut changed_chunk_event: EventWriter<ChangedChunk>,
    // mut gizmos: Option<Gizmos>,
) {
    if !sim_settings.run {
        return;
    }

    #[cfg(feature = "trace")]
    let falling_sands_span = info_span!("falling_sands").entered();

    sim_tick.0 = (sim_tick.0 + 1) % (u32::MAX / 2);

    for (_grid_entity, mut sim_chunks) in &mut grids {
        // sim_swap_buffer.0.clear();

        sim_chunks.margolus_offset += 1;
        sim_chunks.margolus_offset %= 8;

        sim_chunks.spread_updates();

        use rayon::prelude::*;
        let views = sim_chunks.chunk_views();

        // Parallel version
        views.into_par_iter().for_each(|mut block_view| {
            block_view.simulate(*sim_tick);
        });

        // Single threaded version
        // views.into_iter().for_each(|mut chunk_view| {
        //     chunk_view.simulate(*sim_tick);
        // });

        // for (chunk_point, voxel_index) in sim_chunks.sim_updates(&mut
        // sim_swap_buffer.0) {     #[cfg(feature = "trace")]
        //     let update_span = info_span!("update_voxel").entered();

        //     changed_chunk_event.write(ChangedChunk { grid_entity, chunk_point
        // });

        //     let sim_voxel = sim_chunks.get_voxel_from_indices(chunk_point,
        // voxel_index);     // if sim_voxel.is_simulated() {
        //     let point =
        // SimChunks::point_from_chunk_and_voxel_indices(chunk_point,
        // voxel_index);     sim_voxel.simulate(&mut sim_chunks, point,
        // &sim_tick);

        //     // if let Some(gizmos) = gizmos.as_mut() &&
        //     // sim_settings.display_simulated {     gizmos.cuboid(
        //     //         Transform {
        //     //             translation: point.as_vec3() * GRID_SCALE,
        //     //             scale: GRID_SCALE,
        //     //             ..default()
        //     //         },
        //     //         Color::srgb(1.0, 0.0, 0.0),
        //     //     );
        //     // }
        //     // } else {
        //     // if let Some(gizmos) = gizmos.as_mut() &&
        //     // sim_settings.display_checked {     let point =
        //     //
        // SimChunks::point_from_chunk_and_voxel_indices(chunk_point,
        // voxel_index);     //     gizmos.cuboid(
        //     //         Transform {
        //     //             translation: point.as_vec3() * GRID_SCALE,
        //     //             scale: GRID_SCALE,
        //     //             ..default()
        //     //         },
        //     //         Color::srgb(0.0, 0.0, 1.0),
        //     //     );
        //     // }
        //     // }
        // }
    }
}
