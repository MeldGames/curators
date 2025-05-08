use bevy::prelude::*;
use mem_dbg::MemSize;
use serde::{Deserialize, Serialize};

use super::grid::{Grid, Ordering, Scalar};
use super::raycast::Hit;
use crate::voxel::raycast::BoundingVolume3;


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
pub struct VoxelState {
    pub health: i16,
    pub voxel: Voxel,
}

impl From<Voxel> for VoxelState {
    fn from(voxel: Voxel) -> Self {
         Self {
            health: voxel.starting_health(),
            voxel: voxel,
         }
    }
}

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
    Grass,
    Stone,
    Water,
    Base,
}

impl Voxel {
    pub fn starting_health(&self) -> i16 {
        match self {
            Voxel::Air => 0,
            Voxel::Dirt => 1,
            Voxel::Grass => 1,
            Voxel::Stone => 50,
            Voxel::Water => 0,
            Voxel::Base => i16::MAX,
        }
    }
}

impl Voxel {
    pub fn iter() -> impl Iterator<Item = Voxel> {
        [Voxel::Air, Voxel::Dirt, Voxel::Grass, Voxel::Stone, Voxel::Water, Voxel::Base].into_iter()
    }

    pub fn filling(&self) -> bool {
        match self {
            Voxel::Air => false,
            _ => true,
        }
    }

    pub fn pickable(&self) -> bool {
        match self {
            Voxel::Air => false,
            _ => true,
        }
    }

    pub fn breakable(&self) -> bool {
        match self {
            Voxel::Air | Voxel::Base => false,
            _ => true,
        }
    }
}

// TODO: Chunking

/// Simple Voxel grid, zero optimizations done like octrees/chunking/etc.
#[derive(MemSize, Debug, Component, Reflect)]
#[require(
    Name::new("Voxel Grid"),
)]
pub struct VoxelGrid {
    pub grid: Grid,
    pub voxels: Vec<VoxelState>,
    pub surface: Vec<Scalar>,

    // Changed over the last frame.
    changed: Vec<GridChange>,
}

#[derive(MemSize, Debug, Component, Reflect)]
pub struct GridChange {
    pub point: [Scalar; 3],
    pub last_voxel: Voxel,
    pub new_voxel: Voxel,
}

impl VoxelGrid {
    pub fn new([x, y, z]: [Scalar; 3], ordering: Ordering) -> Self {
        let grid = Grid::new([x, y, z], ordering);
        let size = grid.size();
        Self {
            grid,
            voxels: vec![VoxelState::from(Voxel::Air); size as usize],
            surface: Vec::new(),
            changed: Vec::new(),
        }
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
        self.get_voxel_state(point).map(|state| state.voxel)
    }

    pub fn get_voxel_state(&self, point: [Scalar; 3]) -> Option<VoxelState> {
        if !self.in_bounds(point) {
            return None;
        }

        let index = self.grid.linearize(point);
        self.voxels.get(index as usize).copied()
    }

    pub fn voxel(&self, point: [Scalar; 3]) -> Voxel {
        self.voxel_state(point).voxel
    }

    pub fn voxel_state(&self, point: [Scalar; 3]) -> VoxelState {
        if !self.in_bounds(point) {
            panic!("Point out of bounds: {:?}", point);
        }

        let index = self.linearize(point);
        self.linear_voxel_state(index)
    }

    #[inline]
    pub fn linear_voxel_state(&self, index: Scalar) -> VoxelState {
        self.voxels[index as usize]
    }

    pub fn set(&mut self, point: [Scalar; 3], voxel: VoxelState) {
        if !self.in_bounds(point) {
            panic!("Point out of bounds: {:?}", point);
        }

        let index = self.linearize(point);
        let last_voxel = self.linear_voxel_state(index);
        if last_voxel.voxel != voxel.voxel {
            self.changed.push(GridChange { point, last_voxel: last_voxel.voxel, new_voxel: voxel.voxel });
        }

        if voxel.voxel.filling() {
            // if filling, check that neighbors are still in the surface.
        } else {
        }

        self.voxels[index as usize] = voxel;
    }

    pub fn changed(&self) -> impl Iterator<Item = &GridChange> {
        self.changed.iter()
    }

    pub fn clear_changed(&mut self) {
        self.changed.clear();
    }

    pub fn clear_changed_system(mut grids: Query<&mut VoxelGrid>) {
        for mut grid in &mut grids {
            grid.clear_changed();
        }
    }

    #[inline]
    pub fn in_bounds(&self, point: [Scalar; 3]) -> bool {
        self.grid.in_bounds(point)
    }

    #[inline]
    pub fn scaled_bounds(&self) -> Vec3 {
        self.grid.scaled_bounds()
    }

    #[inline]
    pub fn bounds(&self) -> Vec3 {
        self.grid.bounds()
    }

    #[inline]
    pub fn ground_level(&self) -> Scalar {
        (self.grid.y() as f32 / 2.0).ceil() as Scalar
    }

    #[inline]
    pub fn array(&self) -> [Scalar; 3] {
        self.grid.array()
    }

    #[inline]
    pub fn size(&self) -> Scalar {
        self.grid.size()
    }

    // Closest voxel to the surface at a specified x and z.
    // This is a hack compared to a real screenspace raycast.
    pub fn surface_voxel(&self, x: Scalar, z: Scalar) -> Option<(Voxel, Scalar)> {
        for y in (0..self.grid.y()).rev() {
            let voxel = self.voxel([x, y, z]);
            if voxel != Voxel::Air {
                return Some((voxel, y));
            }
        }

        None
    }

    pub fn cast_ray(&self, grid_transform: &GlobalTransform, ray: Ray3d) -> Option<Hit> {
        let inv_matrix = grid_transform.compute_matrix().inverse();
        let local_direction =
            Dir3::new(inv_matrix.transform_vector3(ray.direction.as_vec3())).unwrap();
        let local_origin = inv_matrix.transform_vector3(ray.origin);

        let local_ray = Ray3d { origin: local_origin, direction: local_direction };

        self.cast_local_ray(local_ray)
    }

    /// Cast a ray in the localspace of the voxel grid.
    pub fn cast_local_ray(&self, ray: Ray3d) -> Option<Hit> {
        let volume = BoundingVolume3 { size: self.array().into() };
        for hit in volume.traverse_ray(ray, f32::INFINITY) {
            let voxel = self.voxel(hit.voxel.into());
            if voxel.pickable() {
                return Some(hit);
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
        let grid = VoxelGrid::new([5, 5, 5], Ordering::XYZ);
        assert!(grid.in_bounds([0, 0, 0]));
        assert!(grid.in_bounds([4, 4, 4]));
        assert!(!grid.in_bounds([5, 5, 5]));
        assert!(!grid.in_bounds([5, 0, 0]));

        // Stupid case that should never happen, but just to check:
        let grid = VoxelGrid::new([0, 0, 0], Ordering::XYZ);
        assert!(!grid.in_bounds([0, 0, 0]));
        assert!(!grid.in_bounds([3, 1, 2]));
    }

    #[test]
    pub fn test_size() {
        let grid = VoxelGrid::new([500, 500, 500], Ordering::XYZ);
        println!(
            "grid mem usage: {:?}",
            memory_human_readable(grid.mem_size(mem_dbg::SizeFlags::CAPACITY)),
        );
    }
}
