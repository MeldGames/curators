use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::voxel::simulation::kinds::liquid::{DEFAULT_LIQUID_STATE, LiquidState};

pub fn plugin(app: &mut App) {
    app.register_type::<Voxel>();
    app.register_type::<VoxelMaterials>();
    app.add_systems(Startup, VoxelMaterials::setup);
}

pub type VoxelId = u8;
pub type VoxelData = u8;
pub type VoxelBits = u16;
pub const VOXEL_ID_BITCOUNT: usize = std::mem::size_of::<VoxelId>() * 8;
pub const VOXEL_DATA_BITCOUNT: usize = std::mem::size_of::<VoxelData>() * 8;

#[derive(Reflect, Hash, PartialEq, Eq, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum Voxel {
    Air, // special case "nothing"

    // unbreakable
    Base,
    Barrier,

    // solids
    Dirt,
    Grass,
    Stone,

    // semi-solids (falling sand)
    Sand,

    // liquids
    Water(LiquidState), // TODO: add lateral velocity to remove oscillation?
    Oil(LiquidState),

    // Special
    Fire { voxel_id: VoxelData },
}

// pub struct VoxelData(u16);

#[inline]
pub fn pack_voxel(voxel: Voxel) -> VoxelBits {
    let extra_data: VoxelData = match voxel {
        Voxel::Water(state) | Voxel::Oil(state) => state.bits(),
        Voxel::Fire { voxel_id } => voxel_id,
        _ => 0,
    };

    let data = ((extra_data as VoxelBits) << VOXEL_ID_BITCOUNT) | voxel.id();
    data
}

#[inline]
pub fn unpack_voxel(data: VoxelBits) -> Voxel {
    let id = Voxel::id_from_data(data) & 0xFF;
    let extra_data = ((data >> 8) & 0xFF) as u8;
    match id {
        0 => Voxel::Air,
        1 => Voxel::Base,
        2 => Voxel::Barrier,
        3 => Voxel::Dirt,
        4 => Voxel::Grass,
        5 => Voxel::Stone,
        6 => Voxel::Sand,
        7 => Voxel::Water(LiquidState::from_bits(extra_data)),
        8 => Voxel::Oil(LiquidState::from_bits(extra_data)),
        9 => Voxel::Fire { voxel_id: extra_data },
        _ => panic!("Invalid voxel id: {}", id),
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Interactions {
    /// What voxel this turns into after being burnt.
    /// None means it isn't flammable.
    pub burnt: Option<Voxel>,
}

pub const DEFAULT_INTERACTIONS: Interactions = Interactions { burnt: None };

impl Default for Interactions {
    fn default() -> Self {
        DEFAULT_INTERACTIONS
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct VoxelDefinition {
    pub voxel: Voxel,
    pub name: &'static str,
    /// How does this interaction with falling sands sim (cellular automaton)?
    pub simulation_kind: SimKind,
    /// Should we simulate this voxel?
    pub simulated: bool,
    /// Can we generally collide with this voxel?
    pub collidable: bool,
    /// Should this voxel be rendered?
    pub rendered: bool,
    /// Is this transparent? (should the rendering consider this non-filling)
    pub transparent: bool,
    /// Should raycasts pick this?
    pub pickable: bool,
    /// Can we break this?
    pub breakable: bool,
    /// Initial health of the voxel.
    pub initial_health: i16,
    /// "Density" of voxel, only really important for liquids/gases
    pub density: i8,

    /// Should this voxel cast shadows?
    pub shadow_caster: bool,
    /// Should this voxel receive shadows?
    pub shadow_receiver: bool,

    pub interactions: Interactions,
}

#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Serialize, Deserialize,
)]
pub enum SimKind {
    Solid, // No sim
    SemiSolid,
    Liquid,
    Gas,
    Special,
}

pub const VOXEL_DEFINITIONS: &[&'static VoxelDefinition] = &[
    &VoxelDefinition {
        voxel: Voxel::Air,
        name: "air",
        simulation_kind: SimKind::Gas,
        simulated: false,
        collidable: false,
        rendered: false,
        transparent: true,
        pickable: false,
        breakable: false,
        initial_health: 0,
        density: 0,
        shadow_caster: false,
        shadow_receiver: false,

        interactions: DEFAULT_INTERACTIONS,
    },
    &VoxelDefinition {
        voxel: Voxel::Base,
        name: "base",
        simulation_kind: SimKind::Solid,
        simulated: false,
        collidable: true,
        rendered: true,
        transparent: false,
        pickable: true,
        breakable: false,
        initial_health: 0,
        density: 0,
        shadow_caster: true,
        shadow_receiver: true,

        interactions: DEFAULT_INTERACTIONS,
    },
    &VoxelDefinition {
        voxel: Voxel::Barrier, // base but transparent
        name: "barrier",
        simulation_kind: SimKind::Solid,
        simulated: false,
        collidable: true,
        rendered: false,
        transparent: true,
        pickable: false,
        breakable: false,
        initial_health: 0,
        density: 0,
        shadow_caster: false,
        shadow_receiver: false,

        interactions: DEFAULT_INTERACTIONS,
    },
    &VoxelDefinition {
        voxel: Voxel::Dirt,
        name: "dirt",
        simulation_kind: SimKind::Solid,
        simulated: true,
        collidable: true,
        rendered: true,
        transparent: false,
        pickable: true,
        breakable: true,
        initial_health: 10,
        density: 0,
        shadow_caster: true,
        shadow_receiver: true,

        interactions: DEFAULT_INTERACTIONS,
    },
    &VoxelDefinition {
        voxel: Voxel::Grass,
        name: "grass",
        simulation_kind: SimKind::Solid,
        simulated: false,
        collidable: true,
        rendered: true,
        transparent: false,
        pickable: true,
        breakable: true,
        initial_health: 10,
        density: 0,
        shadow_caster: true,
        shadow_receiver: true,

        interactions: Interactions { burnt: Some(Voxel::Dirt) },
    },
    &VoxelDefinition {
        voxel: Voxel::Stone,
        name: "stone",
        simulation_kind: SimKind::Solid,
        simulated: false,
        collidable: true,
        rendered: true,
        transparent: false,
        pickable: true,
        breakable: true,
        initial_health: 100,
        density: 0,
        shadow_caster: true,
        shadow_receiver: true,

        interactions: DEFAULT_INTERACTIONS,
    },
    &VoxelDefinition {
        voxel: Voxel::Sand,
        name: "sand",
        simulation_kind: SimKind::SemiSolid,
        simulated: true,
        collidable: true,
        rendered: true,
        transparent: false,
        pickable: true,
        breakable: true,
        initial_health: 10,
        density: 0,
        shadow_caster: true,
        shadow_receiver: true,

        interactions: DEFAULT_INTERACTIONS,
    },
    &VoxelDefinition {
        voxel: Voxel::Water(DEFAULT_LIQUID_STATE),
        // voxel: Voxel::Water,
        name: "water",
        simulation_kind: SimKind::Liquid,
        simulated: true,
        collidable: false,
        rendered: true,
        transparent: true,
        pickable: false,
        breakable: true,
        initial_health: 10,
        density: 40,
        shadow_caster: false,
        shadow_receiver: true,

        interactions: DEFAULT_INTERACTIONS,
    },
    &VoxelDefinition {
        voxel: Voxel::Oil(DEFAULT_LIQUID_STATE),
        // voxel: Voxel::Oil,
        name: "oil",
        simulation_kind: SimKind::Liquid,
        simulated: true,
        collidable: false,
        rendered: true,
        transparent: true,
        pickable: false,
        breakable: true,
        initial_health: 10,
        density: 10,
        shadow_caster: false,
        shadow_receiver: true,

        interactions: Interactions { burnt: Some(Voxel::Air) },
    },
    &VoxelDefinition {
        voxel: Voxel::Fire { voxel_id: 0 },
        // voxel: Voxel::Fire,
        name: "fire",
        simulation_kind: SimKind::Special,
        simulated: true,
        collidable: false,
        rendered: true,
        transparent: true,
        pickable: false,
        breakable: true,
        initial_health: 10,
        density: 10,
        shadow_caster: false,
        shadow_receiver: false,

        interactions: DEFAULT_INTERACTIONS,
    },
];

impl Voxel {
    #[inline]
    pub fn iter() -> impl Iterator<Item = Voxel> {
        VOXEL_DEFINITIONS.iter().map(|def| def.voxel)
    }

    #[inline]
    pub fn type_count() -> usize {
        VOXEL_DEFINITIONS.len()
    }

    #[inline]
    pub fn data(self) -> u16 {
        pack_voxel(self)
    }

    #[inline]
    pub fn id(self) -> u16 {
        match self {
            Voxel::Air => 0,
            Voxel::Base => 1,
            Voxel::Barrier => 2,
            Voxel::Dirt => 3,
            Voxel::Grass => 4,
            Voxel::Stone => 5,
            Voxel::Sand => 6,
            Voxel::Water { .. } => 7,
            Voxel::Oil { .. } => 8,
            Voxel::Fire { .. } => 9,
        }
    }

    #[inline]
    pub fn from_data(data: u16) -> Self {
        unpack_voxel(data)
    }

    #[inline]
    pub fn id_from_data(data: u16) -> u16 {
        data & 0xFF
    }

    #[inline]
    pub fn from_id(id: u16) -> Option<Self> {
        VOXEL_DEFINITIONS.get(id as usize).map(|def| def.voxel)
    }

    #[inline]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().trim() {
            "air" => Some(Voxel::Air),

            "base" => Some(Voxel::Base),
            "barrier" => Some(Voxel::Barrier),

            "sand" => Some(Voxel::Sand),

            "dirt" => Some(Voxel::Dirt),
            "grass" => Some(Voxel::Grass),
            "stone" => Some(Voxel::Stone),

            "water" => Some(Voxel::Water(default())),
            "oil" => Some(Voxel::Oil(default())),
            _ => None,
        }
    }

    #[inline]
    pub fn as_name(&self) -> &'static str {
        self.definition().name
    }

    #[inline]
    pub fn definition(self) -> &'static VoxelDefinition {
        VOXEL_DEFINITIONS[self.id() as usize]
    }

    #[inline]
    pub fn starting_health(&self) -> i16 {
        self.definition().initial_health
    }

    #[inline]
    pub fn rendered(self) -> bool {
        self.definition().rendered
    }

    // is this block see-through (rendering)
    #[inline]
    pub fn transparent(self) -> bool {
        self.definition().transparent
    }

    #[inline]
    pub fn pickable(self) -> bool {
        self.definition().pickable
    }

    #[inline]
    pub fn breakable(self) -> bool {
        self.definition().breakable
    }

    #[inline]
    pub fn collidable(self) -> bool {
        self.definition().collidable
    }

    #[inline]
    pub fn is_liquid(self) -> bool {
        self.definition().simulation_kind == SimKind::Liquid
    }

    #[inline]
    pub fn density(self) -> i8 {
        self.definition().density
    }

    #[inline]
    pub fn denser(self, other: Self) -> bool {
        self.density() > other.density()
    }

    #[inline]
    pub fn is_gas(self) -> bool {
        self.definition().simulation_kind == SimKind::Gas
    }

    #[inline]
    pub fn is_simulated(self) -> bool {
        use Voxel::*;
        match self {
            Sand | Dirt | Water { .. } | Oil { .. } => true,
            _ => false,
        }
    }

    #[inline]
    pub fn shadow_caster(self) -> bool {
        self.definition().shadow_caster
    }

    #[inline]
    pub fn shadow_receiver(self) -> bool {
        self.definition().shadow_receiver
    }

    #[inline]
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
                // base_color: Color::BLACK,
                base_color: Color::srgb(0.6, 0.6, 0.6),
                ..default_material
            },
            Voxel::Water { .. } => StandardMaterial {
                perceptual_roughness: 0.5,
                base_color: Color::srgba(10.0 / 225.0, 10.0 / 255.0, 150.0 / 255.0, 0.2),
                reflectance: 0.5,
                alpha_mode: AlphaMode::Blend,
                ..default_material
            },
            Voxel::Oil { .. } => StandardMaterial {
                perceptual_roughness: 0.5,
                reflectance: 0.9,
                base_color: Color::srgba(79.0 / 225.0, 55.0 / 255.0, 39.0 / 255.0, 0.2),
                alpha_mode: AlphaMode::Blend,
                ..default_material
            },
            Voxel::Sand => StandardMaterial {
                perceptual_roughness: 1.0,
                base_color: Color::srgba(0.396, 0.314, 0.113, 1.0),
                ..default_material
            },
            _ => default_material,
        }
    }
}

#[derive(Resource, Clone, Reflect)]
#[reflect(Resource)]
pub struct VoxelMaterials {
    pub base: Handle<StandardMaterial>,
    pub sand: Handle<StandardMaterial>,
    pub dirt: Handle<StandardMaterial>,
    pub grass: Handle<StandardMaterial>,
    pub water: Handle<StandardMaterial>,
    pub oil: Handle<StandardMaterial>,
}

impl VoxelMaterials {
    pub fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            base: materials.add(Voxel::Base.material()),
            dirt: materials.add(Voxel::Dirt.material()),
            sand: materials.add(Voxel::Sand.material()),
            grass: materials.add(Voxel::Grass.material()),
            water: materials.add(Voxel::Water(default()).material()),
            oil: materials.add(Voxel::Oil(default()).material()),
        }
    }

    pub fn get(&self, voxel: Voxel) -> Handle<StandardMaterial> {
        match voxel {
            Voxel::Base => self.base.clone(),
            Voxel::Sand => self.sand.clone(),
            Voxel::Dirt => self.dirt.clone(),
            Voxel::Grass => self.grass.clone(),
            Voxel::Water { .. } => self.water.clone(),
            Voxel::Oil { .. } => self.oil.clone(),
            _ => self.base.clone(),
        }
    }

    pub fn setup(mut commands: Commands, materials: Option<ResMut<Assets<StandardMaterial>>>) {
        if let Some(mut materials) = materials {
            commands.insert_resource(VoxelMaterials::new(&mut *materials));
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Reflect)]
pub struct VoxelChangeset(u16);

impl Default for VoxelChangeset {
    fn default() -> Self {
        Self(0)
    }
}

impl Iterator for VoxelChangeset {
    type Item = Voxel;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 != 0 {
            // `bitset & -bitset` returns a bitset with only the lowest significant bit set
            let t = self.0 & self.0.wrapping_neg();
            let trailing = self.0.trailing_zeros() as usize;
            self.0 ^= t;
            return Voxel::from_id(trailing as u16);
        }

        None
    }
}

impl VoxelChangeset {
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    pub fn set(&mut self, voxel: Voxel) {
        self.0 |= 1 << voxel.id();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn voxel_mem_test() {
        assert_eq!(std::mem::size_of::<Voxel>(), 2);
    }

    #[test]
    fn def_sanity() {
        for (id, &def) in VOXEL_DEFINITIONS.iter().enumerate() {
            assert_eq!(
                def.voxel,
                Voxel::from_id(id as u16).unwrap(),
                "Id for voxel {:?} is not the same as definition index: {} != {}",
                Voxel::from_id(id as u16),
                id,
                def.voxel.id()
            );
            assert_eq!(def.voxel.id(), id as u16);
        }
    }

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
        let mut unique = std::collections::HashSet::new();
        for voxel in Voxel::iter() {
            let name = voxel.as_name();
            if unique.contains(name) {
                panic!("name exists twice: {:?}", name);
            }

            unique.insert(name);
            let from_name = Voxel::from_name(name).unwrap();
            assert_eq!(from_name, voxel);
        }
    }

    #[test]
    fn pack_unpack() {
        for voxel in Voxel::iter() {
            println!("voxel: {:?}", voxel);
            let data = voxel.data();
            println!("data: {:016b}", data);
            let unpacked = Voxel::from_data(data);
            println!("unpacked: {:?}", unpacked);
            assert_eq!(voxel, unpacked);
        }
    }
}
