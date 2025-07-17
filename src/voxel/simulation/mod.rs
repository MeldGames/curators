//! Voxel falling sands implementation.
//!
//! This needs to be relatively fast... going to be a
//! large experiment onto whether we can make this work or not.

use crate::voxel::{Voxel, Voxels};
use bevy::prelude::*;

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

pub fn falling_sands(mut grids: Query<&mut Voxels>, mut updates: Local<Vec<IVec3>>,
    mut sim_tick: ResMut<FallingSandTick>,
    mut ignore: Local<usize>,
) {
    *ignore = (*ignore + 1) % 2; // 60 / 2 ticks per second
    if *ignore != 0 {
        return;
    }

    sim_tick.0 = (sim_tick.0 + 1) % (u32::MAX / 2);

    // const MAX_UPDATE: usize = 1_000_000;
    let mut simulated_counter = 0;
    let mut static_counter = 0;

    for mut grid in &mut grids {
        updates.extend(grid.update_voxels.drain(..));
        updates.sort_by(|a, b| b.y.cmp(&a.y).then(b.x.cmp(&a.x)).then(b.z.cmp(&a.z)));
        updates.dedup();

        while let Some(point) = updates.pop() {
            let sim_voxel = grid.get_voxel(point);
            if sim_voxel.is_simulated() {
                simulated_counter += 1;
            } else {
                static_counter += 1;
            }

            match sim_voxel {
                Voxel::Sand => { // semi-solid
                    simulate_semisolid(&mut grid, point, sim_voxel, &sim_tick);
                },
                Voxel::Water | Voxel::Oil => { // liquids
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

    if simulated_counter > 0 {
        // info!("simulated {} voxels, static {} voxels", simulated_counter, static_counter);
    }
}

pub fn simulate_semisolid(grid: &mut Voxels, point: IVec3, sim_voxel: Voxel, sim_tick: &FallingSandTick) {
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

pub fn simulate_liquid(grid: &mut Voxels, point: IVec3, sim_voxel: Voxel, sim_tick: &FallingSandTick) {
    let swap_criteria = |voxel: Voxel| {
        voxel.is_gas() || (voxel.is_liquid() && sim_voxel.denser(voxel))
    };

    const SWAP_POINTS: [IVec3; 8] =[
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
        for swap_point in SWAP_POINTS.iter().cycle().skip((sim_tick.0 % 4) as usize).take(4) {
            let voxel = grid.get_voxel(IVec3::from(point + swap_point));
            if swap_criteria(voxel) {
                grid.set_voxel(point + swap_point, sim_voxel);
                grid.set_voxel(point, voxel);
                break;
            }
        }
    }
}

pub fn simulate_structured(grid: &mut Voxels, point: IVec3, sim_voxel: Voxel, sim_tick: &FallingSandTick) {
    let below_voxel = grid.get_voxel(point + IVec3::new(0, -1, 0));
    if below_voxel == Voxel::Air {
        const SURROUNDING: [IVec3; 5] = [
            ivec3(-1, 0, 0),
            ivec3(1, 0, 0),
            ivec3(0, 0, -1),
            ivec3(0, 0, 1),
            ivec3(0, 1, 0),
        ];

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
