//! Spawning of fossils and the voxels in the digsite area.
use bevy::prelude::*;
use thiserror::Error;

use crate::voxel::Voxels;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, gen_digsite)
}

#[derive(Event)]
pub struct GenerateDigsite {
    pub digsite: Digsite,
}

pub fn gen_digsite(mut requests: EventReader<GenerateDigsite>, mut voxels: Query<&mut Voxels>) {
    let Ok(voxels) = voxels.single_mut() else {
        return;
    };

    for request in requests.read() {
        if let Err(error) = request.digsite.generate_digsite(&mut voxels) {
            error!("Digsite generation: {:?}", error);
        }
    }
}

#[derive(Component)]
pub struct Digsite {
    /// Bounds of the digsite in the global voxel grid.
    pub start: IVec3,
    pub end: IVec3,

    /// Layers starting from bottom up
    /// Second param is how many blocks thick the layer is.
    pub layers: Vec<(Voxel, i32)>,
    /// Intensity of noise map per layer to drift into other layers.
    pub layer_drift: i32,
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

        let layer_thickness_sum = self.layers.iter().map(|(_, thick)| thick).sum();
        let bounds_height = (self.end.y - self.start.y).abs();

        if layer_thickness_sum >= bounds_height {
            errors.push(GenError::LayerThickness { bounds_height, layer_thickens });
        }

        if errors.len() > 0 {
            Err(errors);
        } else {
            Ok(())
        }
    }

    pub fn generate_digsite(&self, voxels: &mut Voxels) -> Result<(), Vec<GenError>> {
        self.validate()?;

        let (min, max) = self.bounds();
        let mut layer_index = (self.layers.len() - 1) as i32;
        let mut base_layer = false;
        let mut thickness_counter = 0;

        // Set up layers
        for y in (min.y..max.y).rev() {
            let (layer_voxel, layer_thickness) =
                if layer_index < 0 { (Voxel::Base, i32::MAX) } else { self.layers[layer_index] };

            for x in min.x..max.x {
                for z in min.z..max.z {
                    voxels.set_voxel([x, y, z], layer_voxel);
                }
            }

            if thickness_counter > layer_thickness {
                layer_index -= 1;
            }
            thickness_counter += 1;
        }

        // TODO: blob generation
        // create blobs of a voxel randomly with desired depth and largeness

        // TODO: Conform top of the digsite to the terrain noise.
        Ok(())
    }
}
