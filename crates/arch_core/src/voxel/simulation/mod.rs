//! Voxel falling sands implementation.
//!
//! This needs to be relatively fast... going to be a
//! large experiment onto whether we can make this work or not.

use bevy::prelude::*;
#[cfg(feature = "trace")]
use tracing::*;

use crate::voxel::simulation::data::{CHUNK_LENGTH, ChunkPoint, SimChunks};
use crate::voxel::tree::VoxelNode;
use crate::voxel::{Voxel, Voxels};

pub mod data;
pub mod debug_dirty;
pub mod gpu;
pub mod kinds;
pub mod morton;
pub mod rle;
pub mod set;
pub mod view;

#[derive(SystemSet, Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub enum SimStep {
    #[default]
    AddVoxelsToSim,
    PullFromTree,
    FlagDirty,
    Simulate,
    PropagateToTree,
}

impl SimStep {
    pub fn next(&self) -> Self {
        match self {
            SimStep::AddVoxelsToSim => SimStep::PullFromTree,
            SimStep::PullFromTree => SimStep::FlagDirty,
            SimStep::FlagDirty => SimStep::Simulate,
            SimStep::Simulate => SimStep::PropagateToTree,
            SimStep::PropagateToTree => SimStep::AddVoxelsToSim,
        }
    }
}

pub fn plugin(app: &mut App) {
    app.register_type::<FallingSandTick>()
        .register_type::<SimSettings>()
        .register_type::<SimStep>()
        .register_type::<SimRun>();

    app.insert_resource(FallingSandTick(0));
    app.insert_resource(SimSettings::default());

    app.configure_sets(
        FixedPostUpdate,
        (
            SimStep::AddVoxelsToSim.run_if(SimRun::should_step(SimStep::AddVoxelsToSim)),
            SimStep::PullFromTree.run_if(SimRun::should_step(SimStep::PullFromTree)),
            SimStep::FlagDirty.run_if(SimRun::should_step(SimStep::FlagDirty)),
            SimStep::Simulate.run_if(SimRun::should_step(SimStep::Simulate)),
            SimStep::PropagateToTree.run_if(SimRun::should_step(SimStep::PropagateToTree)),
        )
            .chain()
            .run_if(SimRun::should_run),
    );

    app.add_systems(FixedLast, SimRun::advance_step);

    app.add_systems(FixedPostUpdate, add_sand.in_set(SimStep::AddVoxelsToSim))
        .add_systems(FixedPostUpdate, pull_from_tree.in_set(SimStep::PullFromTree))
        .add_systems(FixedPostUpdate, spread_updates.in_set(SimStep::FlagDirty))
        .add_systems(FixedPostUpdate, simulate.in_set(SimStep::Simulate))
        .add_systems(FixedPostUpdate, propagate_to_tree.in_set(SimStep::PropagateToTree));

    app.add_systems(First, sim_settings);

    app.add_systems(Startup, || {
        info!("available parallelism: {:?}", std::thread::available_parallelism());
    });

    app.add_plugins(data::plugin);
    app.add_plugins(debug_dirty::plugin);
}

// Make islands of voxels fall if unsupported.
pub fn islands(mut grids: Query<&mut Voxels>) {}

#[derive(Resource, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[reflect(Resource)]
pub struct FallingSandTick(pub u32);

#[derive(Clone, Default, Debug, PartialEq, Eq, Reflect)]
pub enum SimRun {
    #[default]
    Continuous,
    Granular(SimStep),
}

impl SimRun {
    pub fn should_step(step: SimStep) -> impl Fn(Res<SimSettings>) -> bool {
        move |settings: Res<SimSettings>| -> bool {
            match &settings.step {
                SimRun::Continuous => true,
                SimRun::Granular(granular_step) if step == *granular_step => true,
                _ => false,
            }
        }
    }

    pub fn should_run(settings: Res<SimSettings>) -> bool {
        settings.step == SimRun::Continuous || settings.step_once
    }

    pub fn advance_step(mut settings: ResMut<SimSettings>) {
        let SimSettings {
            step,
            step_once,
            ..
        } = &mut *settings;

        match step {
            SimRun::Granular(step) => {
                if *step_once {
                    *step = step.next();
                    *step_once = false;
                }
            }
            SimRun::Continuous => {
                *step_once = false;
            }
            _ => {},
        }
    }
}

#[derive(Resource, Clone, Reflect)]
#[reflect(Resource)]
pub struct SimSettings {
    /// Run the simulation continuously or granularly.
    pub step: SimRun,

    /// Step the simulation once, this will be flipped after we simulate either 1 frame,
    /// or 1 granular system if [`SimSettings::step_granular`] is set.
    pub step_once: bool,

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
        Self {
            step: SimRun::Continuous,
            // step: SimRun::Granular(default()),
            step_once: false,
            display_simulated: false,
            display_checked: false,
            sim_threads: threads,
        }
    }
}

pub fn sim_settings(mut sim_settings: ResMut<SimSettings>, input: Res<ButtonInput<KeyCode>>) {
    // if input.just_pressed(KeyCode::KeyL) {
    // sim_settings.display_simulated = sim_settings.display_simulated;
    // }

    if input.just_pressed(KeyCode::KeyL) {
        sim_settings.step_once = true;
    }
}

// Pull relevant chunks from the 64tree into our linear array.
pub fn pull_from_tree(
    mut grids: Query<(Entity, &Voxels, &mut SimChunks)>,
    tick: Res<FallingSandTick>,
) {
    // TODO: Stop doing this on every chunk every frame, should only do this on modified chunks.
    for (_grid_entity, voxels, mut sim_chunks) in &mut grids {
        for z in 0..4 {
            for x in 0..4 {
                for y in 0..2 {
                    let chunk_point = IVec3::new(x, y, z);
                    // if !voxels.tree.changed_chunks.contains(&chunk_point) {
                    //     continue;
                    // }

                    let voxels = match voxels.tree.root.get_chunk(chunk_point) {
                        VoxelNode::Solid { voxel, .. } => Some([*voxel; CHUNK_LENGTH]),
                        VoxelNode::Leaf { leaf } => Some(**leaf),
                        _ => None,
                    };

                    if let Some(voxels) = voxels {
                        // info!("pulling chunk to sim: {:?}", chunk_point);
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
                },
                VoxelNode::Leaf { leaf } => {
                    for voxel_index in sim_chunk.modified.iter() {
                        leaf[voxel_index] = sim_chunk.voxels[voxel_index];
                    }

                    // TODO: Set updated neighboring chunks here
                },
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

pub fn spread_updates(mut grids: Query<(Entity, &mut SimChunks)>) {
    for (_grid_entity, mut sim_chunks) in &mut grids {
        // use the current margolus offset to preserve boundary dirtiness
        sim_chunks.spread_updates();

        sim_chunks.margolus_offset += 1;
        sim_chunks.margolus_offset %= 8;
    }
}

pub fn simulate(mut grids: Query<(Entity, &mut SimChunks)>, mut sim_tick: ResMut<FallingSandTick>) {
    #[cfg(feature = "trace")]
    let falling_sands_span = info_span!("falling_sands").entered();

    sim_tick.0 = (sim_tick.0 + 1) % (u32::MAX / 2);

    for (_grid_entity, mut sim_chunks) in &mut grids {
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
    }
}
