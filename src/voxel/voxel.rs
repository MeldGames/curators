use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Reflect, Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Clone, Serialize, Deserialize,
)]
pub enum Voxel {
    Air,
    Dirt,
    Grass,
    Stone,
    Water,
    Base,
}

impl Voxel {
    pub fn starting_health(&self) -> i16 {
        match self {
            Voxel::Air => 0,
            Voxel::Dirt => 10,
            Voxel::Grass => 10,
            Voxel::Stone => 100,
            Voxel::Water => 0,
            Voxel::Base => i16::MAX,
        }
    }
}

impl Voxel {
    pub fn iter() -> impl Iterator<Item = Voxel> {
        [Voxel::Air, Voxel::Dirt, Voxel::Grass, Voxel::Stone, Voxel::Water, Voxel::Base].into_iter()
    }

    pub fn filling(&self) -> bool {
        match self {
            Voxel::Air => false,
            _ => true,
        }
    }

    pub fn transparent(&self) -> bool {
        match self {
            Voxel::Air | Voxel::Water => true,
            _ => false,
        }
    }

    pub fn pickable(&self) -> bool {
        match self {
            Voxel::Air => false,
            _ => true,
        }
    }

    pub fn breakable(&self) -> bool {
        match self {
            Voxel::Air | Voxel::Base => false,
            _ => true,
        }
    }
}

// TODO: Chunking
