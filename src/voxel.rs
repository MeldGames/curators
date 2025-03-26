use bevy::prelude::*;
use mem_dbg::MemSize;
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        todo!()
    }
}

#[derive(
    MemSize, Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Clone, Serialize, Deserialize,
)]
pub enum Voxel {
    Air,
    Dirt,
    Stone,
    Water,
}

/// Simple Voxel grid, zero optimizations done like octrees/etc.
#[derive(MemSize, Debug)]
pub struct VoxelGrid {
    voxels: Vec<Vec<Vec<Voxel>>>,
}

impl VoxelGrid {
    pub fn new(x: usize, y: usize, z: usize) -> Self {
        Self {
            voxels: vec![vec![vec![Voxel::Air; z]; y]; x],
        }
    }

    pub fn get_voxel(&self, x: usize, y: usize, z: usize) -> Option<Voxel> {
        self.voxels
            .get(x)
            .map(|v| v.get(y).map(|v| v.get(z)))
            .flatten()
            .flatten()
            .copied()
    }

    pub fn voxel(&self, x: usize, y: usize, z: usize) -> Voxel {
        self.voxels[x][y][z]
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, voxel: Voxel) {
        self.voxels[x][y][z] = voxel;
    }

    pub fn in_bounds(&self, x: usize, y: usize, z: usize) -> bool {
        x < self.voxels.len() && y < self.voxels[0].len() && z < self.voxels[0][0].len()
    }

    pub fn width(&self) -> usize {
        self.voxels.len()
    }

    pub fn height(&self) -> usize {
        if self.voxels.len() > 0 {
            self.voxels[0].len()
        } else {
            0
        }
    }

    pub fn length(&self) -> usize {
        if self.voxels.len() > 0 {
            if self.voxels[0].len() > 0 {
                self.voxels[0][0].len()
            } else {
                0
            }
        } else {
            0
        }
    }

    // Closest voxel to the surface at a specified x and z.
    // This is a hack compared to a real screenspace raycast.
    pub fn surface_voxel(&self, x: usize, z: usize) -> Option<(Voxel, usize)> {
        for y in (0..self.height()).rev() {
            let voxel = self.voxel(x, y, z);
            if voxel != Voxel::Air {
                return Some((voxel, y));
            }
        }

        None
    }
}

pub fn memory_human_readable(bytes: usize) -> String {
    if bytes > 1_000_000_000 {
        format!("{:.2?}Gb", bytes as f64 / 1_000_000_000.0f64)
    } else if bytes > 1_000_000 {
        format!("{:.2?}Mb", bytes as f64 / 1_000_000.0f64)
    } else if bytes > 1_000 {
        format!("{:.2?}Kb", bytes as f64 / 1_000.0f64)
    } else {
        format!("{:?}b", bytes)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn in_bounds() {
        let grid = VoxelGrid::new(5, 5, 5);
        assert!(grid.in_bounds(0, 0, 0));
        assert!(grid.in_bounds(4, 4, 4));
        assert!(!grid.in_bounds(5, 5, 5));
        assert!(!grid.in_bounds(5, 0, 0));

        // Stupid case that should never happen, but just to check:
        let grid = VoxelGrid::new(0, 0, 0);
        assert!(!grid.in_bounds(0, 0, 0));
        assert!(!grid.in_bounds(3, 1, 2));
    }

    #[test]
    pub fn test_size() {
        let grid = VoxelGrid::new(500, 500, 500);
        println!(
            "grid mem usage: {:?}",
            memory_human_readable(grid.mem_size(mem_dbg::SizeFlags::CAPACITY)),
        );
    }
}
