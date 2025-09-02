use bevy::prelude::*;

use crate::voxel::Voxel;
use crate::voxel::simulation::data::ChunkView;
// use crate::voxel::simulation::kinds::liquid::LiquidState;
use crate::voxel::simulation::{FallingSandTick, SimChunks};

#[inline]
pub fn simulate_semisolid(
    view: &mut ChunkView<'_>,
    chunk_index: usize,
    voxel_index: usize,
    sim_voxel: Voxel,
    sim_tick: FallingSandTick,
) {
    #[cfg(feature = "trace")]
    let simulate_semisolid_span = info_span!("simulate_semisolid").entered();

    const SEMISOLID_CHECKS: [IVec3; 5] = [
        IVec3::NEG_Y,
        IVec3::NEG_Y.saturating_add(IVec3::NEG_X),
        IVec3::NEG_Y.saturating_add(IVec3::X),
        IVec3::NEG_Y.saturating_add(IVec3::NEG_Z),
        IVec3::NEG_Y.saturating_add(IVec3::Z),
    ];

    for &check in SEMISOLID_CHECKS.iter() {
        if let Some((relative_chunk_index, relative_voxel_index, relative_voxel)) =
            view.get_relative_voxel(chunk_index, voxel_index, check)
        {
            if relative_voxel.is_gas() || (check == IVec3::NEG_Y && relative_voxel.is_liquid()) {
                view.set_voxel(relative_chunk_index, relative_voxel_index, sim_voxel);
                view.set_voxel(chunk_index, voxel_index, relative_voxel);
                return;
            }
        }
    }
}
