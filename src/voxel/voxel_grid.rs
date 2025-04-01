use bevy::prelude::*;
use mem_dbg::MemSize;
use serde::{Deserialize, Serialize};

use crate::voxel::raycast::BoundingVolume3;

use super::{grid::{Grid, Scalar}, raycast::Hit};

#[derive(
    MemSize,
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
    Air,
    Dirt,
    Stone,
    Water,
}

/// Simple Voxel grid, zero optimizations done like octrees/etc.
#[derive(MemSize, Debug, Component, Reflect)]
pub struct VoxelGrid {
    pub grid: Grid,
    pub voxels: Vec<Voxel>,
}

impl VoxelGrid {
    pub fn new([x, y, z]: [Scalar; 3]) -> Self {
        let grid = Grid::new([x, y, z]);
        let size = grid.size();
        Self { grid, voxels: vec![Voxel::Air; size as usize] }
    }

    #[inline]
    pub fn linearize(&self, point: [Scalar; 3]) -> Scalar {
        self.grid.linearize(point)
    }

    #[inline]
    pub fn delinearize(&self, i: Scalar) -> [Scalar; 3] {
        self.grid.delinearize(i)
    }

    pub fn point_iter(&self) -> impl Iterator<Item = [Scalar; 3]> {
        self.grid.point_iter()
    }

    pub fn get_voxel(&self, point: [Scalar; 3]) -> Option<Voxel> {
        if !self.in_bounds(point) {
            return None;
        }

        let index = self.grid.linearize(point);
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
        self.grid.in_bounds(point)
    }

    #[inline]
    pub fn array(&self) -> [Scalar; 3] {
        self.grid.array()
    }

    #[inline]
    pub fn size(&self) -> Scalar {
        self.grid.size()
    }

    #[inline]
    pub fn width(&self) -> Scalar {
        self.grid.width()
    }

    #[inline]
    pub fn height(&self) -> Scalar {
        self.grid.height()
    }

    #[inline]
    pub fn length(&self) -> Scalar {
        self.grid.length()
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

    pub fn cast_ray(&self, grid_transform: &Transform, ray: Ray3d) -> Option<Hit> {
        let inv_matrix = grid_transform.compute_matrix().inverse();
        let local_direction = Dir3::new(inv_matrix.transform_vector3(ray.direction.as_vec3())).unwrap();
        let local_origin = inv_matrix.transform_vector3(ray.origin);

        let local_ray = Ray3d {
            origin: local_origin,
            direction: local_direction,
        };

        self.cast_local_ray(local_ray)
    }

    /// Cast a ray in the localspace of the voxel grid.
    pub fn cast_local_ray(&self, ray: Ray3d) -> Option<Hit> {
        let volume = BoundingVolume3 { size: self.array().into() };
        for hit in volume.traverse_ray(ray, f32::INFINITY) {
            let voxel = self.voxel(hit.voxel.into());
            if let Voxel::Air = voxel {
                continue;
            }

            return Some(hit);
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
