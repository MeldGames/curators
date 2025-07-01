use bevy::ecs::schedule::SystemSet;
use bevy::prelude::*;

pub mod aabb;
pub mod object;
pub mod terrain;
pub mod voxel_aabb;

pub use aabb::Aabb;
pub use object::GenerateObjects;
pub use terrain::{Layers, TerrainParams};
pub use voxel_aabb::VoxelAabb;

use crate::{map::terrain::TerrainKind, voxel::Voxel};

pub fn plugin(app: &mut App) {
    app.configure_sets(
        PreUpdate,
        (
            WorldGenSet::Prepare,
            WorldGenSet::Terrain,
            WorldGenSet::Erosion,
            WorldGenSet::Objects,
            WorldGenSet::SurfaceDetails,
            WorldGenSet::Finalize,
        )
            .chain(),
    );

    app.configure_sets(
        Startup,
        (
            WorldGenSet::Prepare,
            WorldGenSet::Terrain,
            WorldGenSet::Erosion,
            WorldGenSet::Objects,
            WorldGenSet::SurfaceDetails,
            WorldGenSet::Finalize,
        )
            .chain(),
    );

    app.add_plugins(terrain::plugin);
    app.add_plugins(object::plugin);

    app.add_systems(Startup, create_basic_map);
}

/// Stages of world generation.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum WorldGenSet {
    /// Sets up the voxel grid or chunk layout.
    Prepare,
    /// Generates base terrain heightmap or 3D features.
    /// See [`terrain`].
    Terrain,
    /// Generates extra mountains.
    Mountains,
    /// Carves out caves or erosion layers.
    Erosion,
    /// Spawns fossils, artifacts, and other buried items.
    /// See [`objects`].
    Objects,
    /// Spawns vegetation, rocks, and surface features.
    SurfaceDetails,
    /// Finalizes anything needed before the game starts.
    Finalize,
}

pub fn create_basic_map(mut commands: Commands) {
    commands.spawn(
        (MapParams {
            terrain: TerrainParams {
                aabb: VoxelAabb { min: IVec3::new(-100, 0, -100), max: IVec3::new(100, 48, 100) },
                kind: TerrainKind::Flat,
                layers: Layers { layers: vec![(0.0, Voxel::Dirt), (0.9, Voxel::Grass)] },
            },
            digsite: DigsiteParams { count: 1 },
        }),
    );
}

pub struct Generated;

#[derive(Component, Clone, Debug)]
pub struct MapParams {
    pub terrain: TerrainParams,
    // pub mountains: Vec<Mountain>,
    pub digsite: DigsiteParams, // How many digsites to generate
}

#[derive(Event, Clone, Debug)]
pub struct DigsiteParams {
    pub count: usize, // how many digsites to create.
}

#[derive(Component, Debug, Clone)]
pub struct Digsite {
    voxel_aabbs: Vec<VoxelAabb>,
}

impl Digsite {
    pub fn voxel_aabbs(&self) -> &[VoxelAabb] {
        &self.voxel_aabbs
    }

    pub fn remove_aabb_overlaps(&mut self) {
        let mut result: Vec<VoxelAabb> = Vec::new();

        // Remove overlaps among the list
        for current in self.voxel_aabbs.drain(..) {
            let mut fragments = vec![current];

            // Remove overlaps with all boxes already in result
            for existing in &result {
                fragments =
                    fragments.into_iter().flat_map(|frag| frag.subtract(existing)).collect();
            }

            // Add non-overlapping fragments to result
            result.extend(fragments);
        }

        self.voxel_aabbs = result;
    }
}
