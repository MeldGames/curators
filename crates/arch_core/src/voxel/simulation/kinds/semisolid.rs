use bevy::prelude::*;

use crate::voxel::Voxel;
use crate::voxel::simulation::kinds::liquid::LiquidState;
use crate::voxel::simulation::{FallingSandTick, SimChunks};

#[inline]
pub fn simulate_semisolid(
    grid: &mut SimChunks,
    point: IVec3,
    sim_voxel: Voxel,
    sim_tick: &FallingSandTick,
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
        let check_point = point + check;
        let voxel = grid.get_voxel(check_point);
        if voxel.is_gas() || (check == IVec3::NEG_Y && voxel.is_liquid()) {
            grid.set_voxel(check_point, sim_voxel);
            grid.set_voxel(point, voxel);
            return;
        }
    }
}
