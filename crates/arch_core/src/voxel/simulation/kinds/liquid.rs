use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::voxel::Voxel;
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
    pub fn as_ivec3(&self) -> IVec3 {
        match self {
            Direction::Left => -IVec3::X,
            Direction::Right => IVec3::X,
            Direction::Forward => -IVec3::Z,
            Direction::Back => IVec3::Z,
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
    grid: &mut SimChunks,
    point: IVec3,
    sim_voxel: Voxel,
    tick: &FallingSandTick,
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

    // fall down
    let below_point = IVec3::from(point + IVec3::NEG_Y);
    let below_voxel = grid.get_voxel(below_point);
    if below_voxel.is_gas() || (below_voxel.is_liquid() && sim_voxel.denser(below_voxel)) {
        liquid_voxel.set_state(new_direction, 32);

        grid.set_voxel(below_point, liquid_voxel.to_voxel());
        grid.set_voxel(point, below_voxel);
        return;
    }

    // check direction diagonal
    {
        let diagonal_direction_point = point + liquid_voxel.direction().as_ivec3() - IVec3::Y;
        let diagonal_direction_voxel = grid.get_voxel(diagonal_direction_point);

        if diagonal_direction_voxel.is_gas() {
            liquid_voxel.set_state(liquid_voxel.direction(), 32);
            grid.set_voxel(diagonal_direction_point, liquid_voxel.to_voxel());
            grid.set_voxel(point, diagonal_direction_voxel);
            return;
        }
    }

    let energy = liquid_voxel.energy();
    // let energy = 32;
    // TODO: add a bit of perceived randomness to direction
    {
        let direction_point = point + liquid_voxel.direction().as_ivec3();
        let direction_voxel = grid.get_voxel(direction_point);

        if direction_voxel.is_gas() && below_voxel.id() == sim_voxel.id() {
            grid.set_voxel(direction_point, liquid_voxel.to_voxel());
            grid.set_voxel(point, direction_voxel);
            return;
        }

        liquid_voxel.set_state(new_direction, energy);
        grid.set_voxel(point, liquid_voxel.to_voxel());

        if energy == 0 {
            let above_voxel = grid.get_voxel(point + IVec3::Y);
            if below_voxel.id() == sim_voxel.id() || above_voxel.id() == sim_voxel.id() {
                // noop
            } else {
                grid.set_voxel(point, Voxel::Air);
            }
        }
    }

    // let above_voxel = grid.get_voxel(point + IVec3::Y);
    // if above_voxel.id() == sim_voxel.id() {
    //     if energy != 32 {
    //         liquid_voxel.set_state(new_direction, 32);
    //         grid.set_voxel(point, liquid_voxel.to_voxel());
    //         return;
    //     }
    // } else {
    //     if energy == 0 {
    //         grid.set_voxel(point, Voxel::Air);
    //     } else {
    //         liquid_voxel.set_state(new_direction, energy - 1);
    //         grid.set_voxel(point, liquid_voxel.to_voxel());
    //     }

    // return;
    // }

    // let energy = liquid_voxel.energy();
    // if energy == 0 {
    //     return;
    // }

    // new direction because we hit a wall
    // liquid_voxel.set_state(new_direction, energy - 1);
    // grid.set_voxel(point, liquid_voxel.to_voxel());
}
