use bevy::prelude::*;

use crate::voxel::Voxel;
use crate::voxel::simulation::data::ChunkView;
// use crate::voxel::simulation::kinds::liquid::LiquidState;
use crate::voxel::simulation::{FallingSandTick, SimChunks};

// pub mod fire;
pub mod liquid;
pub mod semisolid;

impl Voxel {
    #[inline]
    pub fn simulate(
        &self,
        view: &mut ChunkView<'_>,
        chunk_index: usize,
        voxel_index: usize,
        tick: FallingSandTick,
    ) {
        match self {
            Voxel::Sand => {
                semisolid::simulate_semisolid(view, chunk_index, voxel_index, *self, tick);
            },
            // Voxel::Water(..) | Voxel::Oil(..) => {
            //     liquid::simulate_liquid(chunks, point, *self, &tick);
            // },
            // Voxel::Fire { .. } => {
            //     fire::simulate_fire(chunks, point, self, &tick);
            // }
            // Voxel::Dirt => {
            // let point =
            //     SimChunks::point_from_chunk_and_voxel_indices(chunk_point, voxel_index);
            // simulate_structured(&mut grid.sim_chunks, point, sim_voxel, &sim_tick);
            // },
            _ => {}, // no-op
        }
    }
}
