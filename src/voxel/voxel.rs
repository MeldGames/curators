use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub fn plugin(app: &mut App) {
    app.register_type::<Voxel>();
    app.register_type::<VoxelMaterials>();
    app.add_systems(Startup, VoxelMaterials::setup);
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
)]
pub enum Voxel {
    Air = 0, // special case "nothing"

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
    Water, // TODO: add lateral velocity to remove oscillation?
    Oil,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SimKind {
    Solid, // No sim
    SemiSolid,
    Liquid,
    Gas,
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
    },

    &VoxelDefinition {
        voxel: Voxel::Water,
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
    },
    &VoxelDefinition {
        voxel: Voxel::Oil,
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
    pub fn id(self) -> u16 {
        self as u16
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

            "water" => Some(Voxel::Water),
            "oil" => Some(Voxel::Oil),
            _ => None,
        }
    }

    #[inline]
    pub fn as_name(&self) -> &'static str {
        self.definition().name
    }

    #[inline]
    pub fn definition(self) -> &'static VoxelDefinition {
        VOXEL_DEFINITIONS[self as usize]
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
            Sand | Dirt | Water | Oil => true,
            _ => false,
        }
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
                base_color: Color::srgb(0.0 / 225.0, 0.0 / 255.0, 0.0 / 255.0),
                ..default_material
            },
            Voxel::Water => StandardMaterial {
                perceptual_roughness: 0.5,
                base_color: Color::srgba(10.0 / 225.0, 10.0 / 255.0, 150.0 / 255.0, 0.2),
                reflectance: 0.5,
                alpha_mode: AlphaMode::Premultiplied,
                ..default_material
            },
            Voxel::Oil => StandardMaterial {
                perceptual_roughness: 0.5,
                reflectance: 0.9,
                base_color: Color::srgba(79.0 / 225.0, 55.0 / 255.0, 39.0 / 255.0, 0.2),
                alpha_mode: AlphaMode::Premultiplied,
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
            water: materials.add(Voxel::Water.material()),
            oil: materials.add(Voxel::Oil.material()),
        }
    }

    pub fn get(&self, voxel: Voxel) -> Handle<StandardMaterial> {
        match voxel {
            Voxel::Base => self.base.clone(),
            Voxel::Sand => self.sand.clone(),
            Voxel::Dirt => self.dirt.clone(),
            Voxel::Grass => self.grass.clone(),
            Voxel::Water => self.water.clone(),
            Voxel::Oil => self.oil.clone(),
            _ => self.base.clone(),
        }
    }

    pub fn setup(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
        commands.insert_resource(VoxelMaterials::new(&mut materials));
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn def_sanity() {
        for (id, &def) in VOXEL_DEFINITIONS.iter().enumerate() {
            assert_eq!(def.voxel, Voxel::from_id(id as u16).unwrap());
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
}
