//! Spawning of fossils and the voxels in the digsite area.
use bevy::prelude::*;
use noiz::math_noise::Pow2;
use noiz::prelude::*;
use thiserror::Error;

use crate::voxel::{Voxel, Voxels};

pub fn plugin(app: &mut App) {
    app.add_event::<GenerateDigsite>();

    app.add_systems(Update, gen_digsite);
    app.add_systems(Startup, send_test_digsite);
}

#[derive(Event, Debug)]
pub struct GenerateDigsite {
    pub digsite: Digsite,
}

pub fn send_test_digsite(mut writer: EventWriter<GenerateDigsite>) {
    writer.send(GenerateDigsite {
        digsite: Digsite {
            start: IVec3::new(0, 0, 0),
            end: IVec3::new(64, 31, 64),

            layers: Layers { layers: vec![(0.0, Voxel::Dirt), (0.9, Voxel::Grass)] },
        },
    });
}

pub fn gen_digsite(mut requests: EventReader<GenerateDigsite>, mut voxels: Query<&mut Voxels>) {
    let Ok(mut voxels) = voxels.single_mut() else {
        return;
    };

    for request in requests.read() {
        info!("request: {:?}", request);
        if let Err(error) = request.digsite.generate_digsite(&mut voxels) {
            error!("Digsite generation: {:?}", error);
        }
    }
}

#[derive(Debug)]
pub struct Layers {
    layers: Vec<(f32, Voxel)>, // 0..1 range of floats
}

impl Layers {
    pub fn sample_height(&self, sample_height: f32) -> Voxel {
        for (layer_height, layer) in &self.layers {
            if *layer_height < sample_height {
                return *layer;
            }
        }

        Voxel::Air
    }
}

#[derive(Component, Debug)]
pub struct Digsite {
    /// Bounds of the digsite in the global voxel grid.
    pub start: IVec3,
    pub end: IVec3,

    /// Layers starting from bottom up
    /// Second param is how many blocks thick the layer is.
    pub layers: Layers,
}

#[derive(Error, Debug)]
pub enum GenError {
    #[error("layer thickness {layer_thickness} is too deep for the bounds height {bounds_height}")]
    LayerThickness { layer_thickness: i32, bounds_height: i32 },
}

impl Digsite {
    pub fn bounds(&self) -> (IVec3, IVec3) {
        (self.start.min(self.end), self.start.max(self.end))
    }

    pub fn validate(&self) -> Result<(), Vec<GenError>> {
        let mut errors = Vec::new();

        // let layer_thickness_sum = self.layers.iter().map(|(_, thick)| thick).sum();
        // let bounds_height = (self.end.y - self.start.y).abs();
        //
        // if layer_thickness_sum >= bounds_height {
        // errors.push(GenError::LayerThickness { bounds_height, layer_thickens });
        // }

        if errors.len() > 0 { Err(errors) } else { Ok(()) }
    }

    pub fn generate_digsite(&self, voxels: &mut Voxels) -> Result<(), Vec<GenError>> {
        info!("generating digsite !!!");
        self.validate()?;

        let (min, max) = self.bounds();

        let mut layer_noise = basic_noise();
        layer_noise.set_seed(1);
        layer_noise.set_period(50.0);

        // Set up layers
        for x in min.x..max.x {
            for z in min.z..max.z {
                let coord_height = layer_noise.sample(Vec2::new(x as f32, z as f32));
                info!("coord_height: {:?}", coord_height);
                let coord_height = coord_height as i32;

                for y in min.y..coord_height {
                    let range_height = y as f32 / (coord_height - min.y) as f32;
                    let voxel = self.layers.sample_height(range_height);
                    voxels.set_voxel(IVec3::new(x, y, z), voxel);
                }
            }
        }

        // TODO: blob generation
        // create blobs of a voxel randomly with desired depth and largeness

        // TODO: Conform top of the digsite to the terrain noise.
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
                    start: 20.0,
                    end: 50.0,
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
