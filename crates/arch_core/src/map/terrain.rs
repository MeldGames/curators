//! Spawning of fossils and the voxels in the digsite area.
use bevy::prelude::*;
use noiz::math_noise::Pow2;
use noiz::prelude::*;
use thiserror::Error;

use crate::map::{MapParams, VoxelAabb, WorldGenSet};
use crate::voxel::{Voxel, Voxels};

pub fn plugin(app: &mut App) {
    info!("terrain plugin !!!!!");
    app.add_systems(PreUpdate, gen_terrain.in_set(WorldGenSet::Terrain));
}

pub fn gen_terrain(
    mut map: Query<&mut MapParams, Changed<MapParams>>,
    mut voxels: Query<&mut Voxels>,
) {
    let Ok(mut map) = map.single_mut() else {
        return;
    };

    let Ok(mut voxels) = voxels.single_mut() else {
        return;
    };

    if let Err(errs) = map.terrain.apply(&mut voxels) {
        error!("failed to generate terrain: {:?}", errs);
    } else {
        info!("generated terrain: {:?}", map.terrain);
    }
}

#[derive(Debug, Clone)]
pub struct Layers {
    pub layers: Vec<(f32, Voxel)>, // 0..1 range of floats
}

impl Layers {
    pub fn sample_height(&self, sample_height: f32) -> Voxel {
        for (layer_height, layer) in self.layers.iter().rev() {
            if *layer_height < sample_height {
                return *layer;
            }
        }

        Voxel::Air
    }
}

#[derive(Event, Clone, Debug)]
pub struct TerrainParams {
    /// Bounds of the terrain in the global voxel grid.
    pub aabb: VoxelAabb,

    /// Layers starting from bottom up
    /// Second param is how many blocks thick the layer is.
    pub layers: Layers,

    pub kind: TerrainKind,
}

#[derive(Clone, Debug)]
pub enum TerrainKind {
    Flat,
    Hilly,
    SimulationBox,
}

#[derive(Error, Debug)]
pub enum GenError {
    #[error("layer thickness {layer_thickness} is too deep for the bounds height {bounds_height}")]
    LayerThickness { layer_thickness: i32, bounds_height: i32 },
}

impl TerrainParams {
    pub fn apply(&self, voxels: &mut Voxels) -> Result<(), Vec<GenError>> {
        info!("generating digsite: {self:?}");

        match &self.kind {
            TerrainKind::Flat => self.flat(voxels),
            TerrainKind::Hilly => self.hilly(voxels),
            TerrainKind::SimulationBox => self.simulation_box(voxels),
        }
    }

    pub fn flat(&self, voxels: &mut Voxels) -> Result<(), Vec<GenError>> {
        let min = self.aabb.min;
        let max = self.aabb.max;

        for x in min.x..max.x {
            for z in min.z..max.z {
                let bounds_height = max.y - min.y;
                let coord_height = bounds_height;

                for y in min.y..coord_height {
                    let range_height = y as f32 / (coord_height - min.y) as f32;
                    let voxel = self.layers.sample_height(range_height);
                    voxels.set_voxel(IVec3::new(x, y, z), voxel);
                }

                voxels.set_voxel(IVec3::new(x, min.y, z), Voxel::Base);
            }
        }

        Ok(())
    }

    pub fn hilly(&self, voxels: &mut Voxels) -> Result<(), Vec<GenError>> {
        let min = self.aabb.min;
        let max = self.aabb.max;

        let mut layer_noise = basic_noise();
        layer_noise.set_seed(1);
        layer_noise.set_period(800.0);

        // Set up layers
        for x in min.x..max.x {
            for z in min.z..max.z {
                let bounds_height = max.y - min.y;
                let removed = layer_noise.sample(Vec2::new(x as f32, z as f32));
                let coord_height = bounds_height - removed as i32;

                for y in min.y..coord_height {
                    let range_height = y as f32 / (coord_height - min.y) as f32;
                    // info!("range_height: {:?}", range_height);
                    let voxel = self.layers.sample_height(range_height);
                    voxels.set_voxel(IVec3::new(x, y, z), voxel);
                }

                voxels.set_voxel(IVec3::new(x, min.y, z), Voxel::Base);
            }
        }

        // TODO: blob generation
        // create blobs of a voxel randomly with desired depth and largeness

        // TODO: Conform top of the digsite to the terrain noise.
        Ok(())
    }

    pub fn simulation_box(&self, voxels: &mut Voxels) -> Result<(), Vec<GenError>> {
        let min = self.aabb.min;
        let max = self.aabb.max;

        voxels.set_voxel_aabb(
            VoxelAabb {
                min: IVec3::new(self.aabb.min.x, self.aabb.min.y, self.aabb.min.z),
                max: IVec3::new(self.aabb.max.x, self.aabb.min.y + 2, self.aabb.max.z),
            },
            Voxel::Base,
        );

        // for x in min.x..=max.x {
        //     for z in min.z..=max.z {
        //         for y in min.y..=max.y {
        //             let box_voxel = if y == min.y { Voxel::Base } else {
        // Voxel::Barrier };

        //             if x == min.x
        //                 || x == max.x
        //                 || y == min.y
        //                 || y == max.y
        //                 || z == min.z
        //                 || z == max.z
        //             {
        //                 voxels.set_voxel(IVec3::new(x, y, z), box_voxel);
        //             }
        //         }
        //     }
        // }

        Ok(())
    }
}

pub fn basic_noise() -> impl SampleableFor<Vec2, f32> + ScalableNoise + SeedableNoise {
    Noise {
        noise: Masked(
            (
                LayeredNoise::new(
                    NormedByDerivative::<f32, EuclideanLength, PeakDerivativeContribution>::default()
                        .with_falloff(0.3),
                    Persistence(0.6),
                    FractalLayers {
                        layer: Octave(BlendCellGradients::<
                            SimplexGrid,
                            SimplecticBlend,
                            QuickGradients,
                            true,
                        >::default()),
                        lacunarity: 1.8,
                        amount: 8,
                    },
                ),
                SNormToUNorm,
                RemapCurve::<Lerped<f32>, f32, false>::from(Lerped {
                    start: 0.0,
                    end: 48.0,
                }),
            ),
            (
                MixCellGradients::<OrthoGrid, Smoothstep, QuickGradients>::default(),
                SNormToUNorm,
                Pow2,
                RemapCurve::<Lerped<f32>, f32, false>::from(Lerped {
                    start: 0.5f32,
                    end: 1.0,
                }),
            ),
        ),
        ..default()
    }

    // Here's another one you can try:
    // Noise {
    //     noise: LayeredNoise::new(
    //         NormedByDerivative::<f32, EuclideanLength,
    // PeakDerivativeContribution>::default()
    // .with_falloff(0.3),         Persistence(0.6),
    //         FractalLayers {
    //             layer: Octave(MixCellGradients::<
    //                 OrthoGrid,
    //                 Smoothstep,
    //                 QuickGradients,
    //                 true,
    //             >::default()),
    //             lacunarity: 1.8,
    //             amount: 8,
    //         },
    //     ),
    //     ..default()
    // }
}
