use bevy::prelude::*;

use crate::voxel::Voxel;
use crate::voxel::simulation::data::{delinearize, linearize, ChunkView};
// use crate::voxel::simulation::kinds::liquid::LiquidState;
use crate::voxel::simulation::{FallingSandTick, SimChunks};

// pub mod fire;
pub mod liquid;
pub mod semisolid;

#[derive(Debug, Copy, Clone)]
pub struct VoxelPosition {
    pub chunk_point: IVec3,
    pub voxel_point: IVec3,
    pub chunk_index: usize,
    pub voxel_index: usize,
}

impl VoxelPosition {
    pub fn from_indices(chunk_index: usize, voxel_index: usize) -> Self {
        let chunk_point = ChunkView::delinearize_chunk(chunk_index);
        let voxel_point = delinearize(voxel_index);
        Self {
            chunk_index, voxel_index, chunk_point, voxel_point,
        }
    }

    pub fn from_points(chunk_point: IVec3, voxel_point: IVec3) -> Self {
        let chunk_index = ChunkView::linearize_chunk(chunk_point);
        let voxel_index = linearize(voxel_point);
        Self {
            chunk_index, voxel_index, chunk_point, voxel_point,
        }
    }

}

impl Voxel {
    #[inline]
    pub fn simulate(
        &self,
        view: &mut ChunkView<'_>,
        voxel_position: VoxelPosition,
        tick: FallingSandTick,
    ) {
        match self {
            Voxel::Sand => {
                semisolid::simulate_semisolid(view, voxel_position, *self, tick);
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
