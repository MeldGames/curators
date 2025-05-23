use bevy::prelude::*;
use num_derive::*;
use num_traits::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};

#[derive(
    Reflect,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Copy,
    Clone,
    Serialize,
    Deserialize,
    FromPrimitive,
    ToPrimitive,
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

    pub fn id(self) -> u16 {
        self.to_u16().unwrap()
    }

    pub fn from_id(id: u16) -> Option<Self> {
        Self::from_u16(id)
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().trim() {
            "air" => Some(Voxel::Air),
            "dirt" => Some(Voxel::Dirt),
            "grass" => Some(Voxel::Grass),
            "water" => Some(Voxel::Water),
            "stone" => Some(Voxel::Stone),
            "base" => Some(Voxel::Base),
            _ => None,
        }
    }

    pub fn filling(self) -> bool {
        match self {
            Voxel::Air => false,
            _ => true,
        }
    }

    pub fn transparent(self) -> bool {
        match self {
            Voxel::Air | Voxel::Water => true,
            _ => false,
        }
    }

    pub fn pickable(self) -> bool {
        match self {
            Voxel::Air => false,
            _ => true,
        }
    }

    pub fn breakable(self) -> bool {
        match self {
            Voxel::Air | Voxel::Base => false,
            _ => true,
        }
    }
}

// TODO: Chunking
