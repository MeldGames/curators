use arch_core::bevy;
use arch_core::voxel::{Voxel, Voxels};
use arch_core::sdf::Sdf;
use bevy::prelude::*;

pub mod falling_sands;
pub mod surface_net;

/// Criterion bench setup
#[derive(Copy, Clone, Debug)]
pub struct MeasurementSetup {
    pub measurement_time: std::time::Duration,
    pub sample_size: usize,
}

impl Default for MeasurementSetup {
    fn default() -> Self {
        Self {
            measurement_time: std::time::Duration::from_secs(10),
            sample_size: 100,
        }
    }
}

/// Set up of a voxel grid for a benchmark
pub struct VoxelSetup {
    /// Size of the voxel grid
    pub voxel_size: IVec3,
    /// Paint voxels in the world each step: (center, brush, voxel)
    pub brushes: Vec<(IVec3, Box<dyn Sdf + Send + Sync>, Voxel)>,
}

impl Default for VoxelSetup {
    fn default() -> Self {
        Self {
            voxel_size: IVec3::splat(256),
            brushes: Vec::new(),
        }
    }
}

impl VoxelSetup {
    pub fn new_voxels(&self) -> Voxels {
        Voxels::new(self.voxel_size)
    }

    pub fn apply_brushes(&self, voxels: &mut Voxels) {
        for (center, brush, voxel) in &self.brushes {
            voxels.set_voxel_brush(*center, &**brush, *voxel);
        }
    }

    pub fn new_with_applied_brushes(&self) -> Voxels {
        let mut voxels = self.new_voxels();
        self.apply_brushes(&mut voxels);
        voxels
    }
}