use bevy::prelude::*;
use num_derive::*;
use num_traits::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};

pub fn plugin(app: &mut App) {
    app.register_type::<Voxel>();
}

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
    Sand,
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
            Voxel::Sand => 10,
            Voxel::Grass => 10,
            Voxel::Stone => 100,
            Voxel::Water => 0,
            Voxel::Base => i16::MAX,
        }
    }
}

impl Voxel {
    pub fn iter() -> impl Iterator<Item = Voxel> {
        [
            Voxel::Air,
            Voxel::Dirt,
            Voxel::Sand,
            Voxel::Grass,
            Voxel::Stone,
            Voxel::Water,
            Voxel::Base,
        ]
        .into_iter()
    }

    pub fn type_count() -> usize {
        Self::iter().count()
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
            "sand" => Some(Voxel::Sand),
            "grass" => Some(Voxel::Grass),
            "water" => Some(Voxel::Water),
            "stone" => Some(Voxel::Stone),
            "base" => Some(Voxel::Base),
            _ => None,
        }
    }

    pub fn as_name(&self) -> &'static str {
        match self {
            Voxel::Air => "air",
            Voxel::Dirt => "dirt",
            Voxel::Sand => "sand",
            Voxel::Grass => "grass",
            Voxel::Water => "water",
            Voxel::Stone => "stone",
            Voxel::Base => "base",
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

    pub fn collidable(self) -> bool {
        match self {
            Voxel::Air | Voxel::Water => false,
            _ => true,
        }
    }

    pub fn material(self) -> StandardMaterial {
        let default_material = StandardMaterial {
            perceptual_roughness: 1.0,
            reflectance: 0.1,
            base_color: Color::WHITE,
            ..default()
        };
        match self {
            Voxel::Dirt => StandardMaterial {
                perceptual_roughness: 1.0,
                base_color: Color::srgb(79.0 / 225.0, 55.0 / 255.0, 39.0 / 255.0),
                ..default_material
            },
            Voxel::Grass => StandardMaterial {
                perceptual_roughness: 1.0,
                base_color: Color::srgb(124.0 / 225.0, 252.0 / 255.0, 0.0 / 255.0),
                ..default_material
            },
            Voxel::Base => StandardMaterial {
                perceptual_roughness: 1.0,
                base_color: Color::srgb(0.0 / 225.0, 0.0 / 255.0, 0.0 / 255.0),
                ..default_material
            },
            Voxel::Water => StandardMaterial {
                perceptual_roughness: 0.5,
                base_color: Color::srgba(10.0 / 225.0, 10.0 / 255.0, 150.0 / 255.0, 0.2),
                alpha_mode: AlphaMode::Premultiplied,
                ..default_material
            },
            _ => default_material,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn id_sanity() {
        assert_eq!(Voxel::Air.id(), 0);

        for voxel in Voxel::iter() {
            let voxel_id = voxel.id();
            let from_id = Voxel::from_id(voxel_id).unwrap();
            assert_eq!(from_id, voxel);
        }
    }

    #[test]
    fn name_sanity() {
        for voxel in Voxel::iter() {
            let name = voxel.as_name();
            let from_name = Voxel::from_name(name).unwrap();
            assert_eq!(from_name, voxel);
        }
    }
}
