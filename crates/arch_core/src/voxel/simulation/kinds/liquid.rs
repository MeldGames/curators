use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::voxel::Voxel;
use crate::voxel::kinds::VoxelPosition;
use crate::voxel::simulation::data::ChunkView;
use crate::voxel::simulation::{FallingSandTick, SimChunks};

#[derive(Copy, Clone, Debug)]
pub enum LiquidVoxel {
    Water(LiquidState),
    Oil(LiquidState),
}

impl LiquidVoxel {
    #[inline]
    pub fn state(&self) -> &LiquidState {
        match self {
            Self::Water(state) | Self::Oil(state) => state,
        }
    }

    #[inline]
    pub fn state_mut(&mut self) -> &mut LiquidState {
        match self {
            Self::Water(state) | Self::Oil(state) => state,
        }
    }

    #[inline]
    pub fn direction(&self) -> Direction {
        self.state().direction()
    }

    #[inline]
    pub fn energy(&self) -> u8 {
        self.state().energy()
    }

    #[inline]
    pub fn from_voxel(voxel: Voxel) -> Self {
        match voxel {
            Voxel::Water(state) => Self::Water(state),
            Voxel::Oil(state) => Self::Oil(state),
            _ => panic!("Voxel was not a liquid voxel: {:?}", voxel),
        }
    }

    #[inline]
    pub fn to_voxel(self) -> Voxel {
        match self {
            Self::Water(state) => Voxel::Water(state),
            Self::Oil(state) => Voxel::Oil(state),
        }
    }

    #[inline]
    pub fn set_state(&mut self, direction: Direction, energy: u8) {
        self.state_mut().set(direction, energy);
    }
}

// packed into a u8
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize)]
pub struct LiquidState(u8);

pub const DEFAULT_LIQUID_STATE: LiquidState = LiquidState::new(Direction::Forward, 32);

impl Default for LiquidState {
    fn default() -> Self {
        DEFAULT_LIQUID_STATE
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Direction {
    Left,
    Right,
    Forward,
    Back,
}

impl Direction {
    #[inline]
    pub const fn as_ivec3(&self) -> IVec3 {
        match self {
            Direction::Left => IVec3::NEG_X,
            Direction::Right => IVec3::X,
            Direction::Forward => IVec3::NEG_Z,
            Direction::Back => IVec3::Z,
        }
    }

    #[inline]
    pub const fn directions() -> [Direction; 4] {
        [Direction::Left, Direction::Right, Direction::Forward, Direction::Back]
    }

    #[inline]
    pub const fn index(&self) -> usize {
        match self {
            Direction::Left => 0,
            Direction::Right => 1,
            Direction::Forward => 2,
            Direction::Back => 3,
        }
    }
}

impl LiquidState {
    #[inline]
    pub fn direction(&self) -> Direction {
        let dir_bits = self.0 & 0b0000_0011;
        let dir = match dir_bits {
            0b00 => Direction::Left,
            0b01 => Direction::Right,
            0b10 => Direction::Forward,
            0b11 => Direction::Back,
            _ => unreachable!(),
        };
        dir
    }

    #[inline]
    pub fn energy(&self) -> u8 {
        (self.0 & 0b1111_1100) >> 2
    }

    #[inline]
    pub fn set(&mut self, direction: Direction, energy: u8) {
        self.0 = Self::pack(direction, energy);
    }

    #[inline]
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    #[inline]
    pub const fn bits(&self) -> u8 {
        self.0
    }

    #[inline]
    pub const fn new(direction: Direction, energy: u8) -> Self {
        Self(Self::pack(direction, energy))
    }

    #[inline]
    pub const fn pack(direction: Direction, energy: u8) -> u8 {
        let dir_bits = match direction {
            Direction::Left => 0b00,
            Direction::Right => 0b01,
            Direction::Forward => 0b10,
            Direction::Back => 0b11,
        };

        let energy_bits = (energy & 0b0011_1111) << 2;
        dir_bits | energy_bits
    }
}

#[inline]
pub fn simulate_liquid(
    view: &mut ChunkView<'_>,
    voxel_position: VoxelPosition,
    sim_voxel: Voxel,
    tick: FallingSandTick,
) {
    #[cfg(feature = "trace")]
    let simulate_liquid_span = info_span!("simulate_liquid").entered();

    let mut liquid_voxel = LiquidVoxel::from_voxel(sim_voxel);

    let new_direction = match tick.0 % 4 {
        0 => Direction::Left,
        1 => Direction::Right,
        2 => Direction::Forward,
        3 => Direction::Back,
        _ => unreachable!(),
    };

    let swappable = |target: Voxel, current: Voxel| {
        target.is_gas() || (target.is_liquid() && current.denser(target))
    };

    const STARTING_ENERGY: u8 = 16;

    // fall down
    // let below_point = IVec3::from(point + IVec3::NEG_Y);
    // let below_voxel = view.get_relative_voxel(below_point);
    // if swappable(below_voxel, sim_voxel) {
    // liquid_voxel.set_state(new_direction, STARTING_ENERGY);
    //
    // grid.set_voxel(below_point, liquid_voxel.to_voxel());
    // grid.set_voxel(point, below_voxel);
    // return;
    // }
    //
    // let direction_voxels: [Voxel; 4] =
    // Direction::directions().map(|d| grid.get_voxel(point + d.as_ivec3()));
    // let open = direction_voxels.iter().filter(|v| **v == Voxel::Air).count();
    // let mix = direction_voxels.iter().filter(|v| v.is_liquid() &&
    // sim_voxel.denser(**v)).count();
    //
    // if open == 0 && mix == 0 {
    // return;
    // }
    //
    // let mut energy = liquid_voxel.energy();
    // let energy = 32;
    // {
    // let direction_point = point + liquid_voxel.direction().as_ivec3();
    // let direction_voxel = direction_voxels[liquid_voxel.direction().index()];
    //
    // if energy == 0 {
    // grid.set_voxel(point, Voxel::Air);
    // return;
    // if below_voxel.id() == sim_voxel.id() {
    //     grid.set_voxel(point, Voxel::Air);
    //     return;
    // } else {
    //     // check every voxel this voxel could've come from.
    //     // type, then don't despawn
    //     return;
    // }
    // }
    //
    // evaporation
    // if open > 0 {
    // let rate = match open {
    // 1 => 4,
    // 2 => 2,
    // 3 => 1,
    // 4 => 1,
    // _ => unreachable!(),
    // };
    //
    // energy = if tick.0 % rate == 0 { energy.saturating_sub(1) } else { energy
    // }; let new_energy = ;
    // liquid_voxel.set_state(liquid_voxel.direction(), new_energy);
    // grid.set_voxel(point, liquid_voxel.to_voxel());
    // }
    //
    // energy = energy.saturating_sub(1);
    //
    // water tension
    // if below_voxel.id() == sim_voxel.id() {
    // prioritize previous direction
    // if swappable(direction_voxel, sim_voxel) {
    // liquid_voxel.set_state(liquid_voxel.direction(), energy);
    // grid.set_voxel(direction_point, liquid_voxel.to_voxel());
    // grid.set_voxel(point, direction_voxel);
    // return;
    // } else {
    // check if there is an open direction to move next time and lose energy
    // const DIRECTIONS: [Direction; 4] =
    // [Direction::Left, Direction::Forward, Direction::Right, Direction::Back];
    // for direction in DIRECTIONS.iter().cycle().skip((tick.0 % 4) as
    // usize).take(4) { if swappable(direction_voxels[direction.index()],
    // sim_voxel) { try a new direction next time.
    // liquid_voxel.set_state(*direction, energy);
    // grid.set_voxel(point, liquid_voxel.to_voxel());
    // return;
    // }
    // }
    //
    // unreachable!("Voxels without an open direction should've been short
    // circuited earlier"); }
    // } else {
    // }
    // }
}
