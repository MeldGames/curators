use bevy::prelude::*;
use mem_dbg::MemSize;
use serde::{Deserialize, Serialize};

#[derive(
    MemSize, Reflect, Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Clone, Serialize, Deserialize,
)]
pub enum Voxel {
    Air,
    Dirt,
    Stone,
    Water,
}

pub type Scalar = u32;

/// Simple Voxel grid, zero optimizations done like octrees/etc.
#[derive(MemSize, Debug, Component, Reflect)]
pub struct VoxelGrid {
    array: [Scalar; 3],
    strides: [Scalar; 3],
    size: Scalar,

    pub voxels: Vec<Voxel>,
}

impl VoxelGrid {
    pub fn new([x, y, z]: [Scalar; 3]) -> Self {
        let size = x * y * z;
        Self { array: [x, y, z], strides: [1, x, x * y], size, voxels: vec![Voxel::Air; size as usize] }
    }

    #[inline]
    pub fn linearize(&self, point: [Scalar; 3]) -> Scalar {
        point[0] + self.strides[1].wrapping_mul(point[1]) + self.strides[2].wrapping_mul(point[2])
    }

    #[inline]
    pub fn delinearize(&self, mut i: Scalar) -> [Scalar; 3] {
        let z = i / self.strides[2];
        i -= z * self.strides[2];
        let y = i / self.strides[1];
        let x = i % self.strides[1];
        [x, y, z]
    }

    pub fn get_voxel(&self, point: [Scalar; 3]) -> Option<Voxel> {
        if !self.in_bounds(point) {
            return None;
        }

        let index = self.linearize(point);
        self.voxels.get(index as usize).copied()
    }

    pub fn voxel(&self, point: [Scalar; 3]) -> Voxel {
        if !self.in_bounds(point) {
            panic!("Point out of bounds: {:?}", point);
        }

        let index = self.linearize(point);
        self.voxels[index as usize]
    }

    pub fn set(&mut self, point: [Scalar; 3], voxel: Voxel) {
        if !self.in_bounds(point) {
            panic!("Point out of bounds: {:?}", point);
        }

        let index = self.linearize(point);
        self.voxels[index as usize] = voxel;
    }

    #[inline]
    pub fn in_bounds(&self, point: [Scalar; 3]) -> bool {
        point[0] < self.width() && point[1] < self.height() && point[2] < self.length()
    }

    #[inline]
    pub fn array(&self) -> [Scalar; 3] {
        self.array
    }

    #[inline]
    pub fn size(&self) -> Scalar {
        self.size
    }

    #[inline]
    pub fn width(&self) -> Scalar {
        self.array[0]
    }

    #[inline]
    pub fn height(&self) -> Scalar {
        self.array[1]
    }

    #[inline]
    pub fn length(&self) -> Scalar {
        self.array[2]
    }

    // Closest voxel to the surface at a specified x and z.
    // This is a hack compared to a real screenspace raycast.
    pub fn surface_voxel(&self, x: Scalar, z: Scalar) -> Option<(Voxel, Scalar)> {
        for y in (0..self.height()).rev() {
            let voxel = self.voxel([x, y, z]);
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
        let grid = VoxelGrid::new([5, 5, 5]);
        assert!(grid.in_bounds([0, 0, 0]));
        assert!(grid.in_bounds([4, 4, 4]));
        assert!(!grid.in_bounds([5, 5, 5]));
        assert!(!grid.in_bounds([5, 0, 0]));

        // Stupid case that should never happen, but just to check:
        let grid = VoxelGrid::new([0, 0, 0]);
        assert!(!grid.in_bounds([0, 0, 0]));
        assert!(!grid.in_bounds([3, 1, 2]));
    }

    #[test]
    pub fn test_size() {
        let grid = VoxelGrid::new([500, 500, 500]);
        println!(
            "grid mem usage: {:?}",
            memory_human_readable(grid.mem_size(mem_dbg::SizeFlags::CAPACITY)),
        );
    }
}
