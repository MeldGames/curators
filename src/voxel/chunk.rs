use bevy::ecs::world::{OccupiedEntry, VacantEntry};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use super::raycast::Hit;
use crate::voxel::Voxel;
use crate::voxel::raycast::BoundingVolume3;

pub type Scalar = i32;

pub mod unpadded {
    use super::Scalar;

    pub const SIZE: usize = 62;
    pub const Z_STRIDE: usize = 1;
    pub const X_STRIDE: usize = SIZE;
    pub const Y_STRIDE: usize = SIZE * SIZE;
    pub const ARR_STRIDE: usize = SIZE * SIZE * SIZE;

    // Padded linearize point into a 62^3 XZY array
    #[inline]
    pub fn linearize([x, y, z]: [Scalar; 3]) -> usize {
        z as usize + x as usize * X_STRIDE + y as usize * Y_STRIDE
    }

    // Delinearize point into a 62^3 array
    #[inline]
    pub fn delinearize(mut index: usize) -> [Scalar; 3] {
        let y = index / Y_STRIDE;
        index -= y * Y_STRIDE;
        let x = index / X_STRIDE;
        let z = index % X_STRIDE;
        [x as Scalar, y as Scalar, z as Scalar]
    }
}

pub mod padded {
    use super::Scalar;

    pub const SIZE: usize = super::unpadded::SIZE + 2;
    pub const Z_STRIDE: usize = 1;
    pub const X_STRIDE: usize = SIZE;
    pub const Y_STRIDE: usize = SIZE * SIZE;
    pub const ARR_STRIDE: usize = SIZE * SIZE * SIZE;

    // Padded linearize point into a 64^3 XZY array
    #[inline]
    pub fn linearize([x, y, z]: [Scalar; 3]) -> usize {
        z as usize + x as usize * X_STRIDE + y as usize * Y_STRIDE
    }

    // Delinearize point into a 64^3 array
    #[inline]
    pub fn delinearize(mut index: usize) -> [Scalar; 3] {
        let y = index / Y_STRIDE;
        index -= y * Y_STRIDE;
        let x = index / X_STRIDE;
        let z = index % X_STRIDE;
        [x as Scalar, y as Scalar, z as Scalar]
    }
}

/// Single voxel chunk, 64^3 (1 padding on the edges for meshing)
#[derive(Debug, Component)]
#[require(Name::new("Voxel Chunk"))]
pub struct VoxelChunk {
    pub voxels: Vec<u16>, // unpadded::ARR_STRIDE length

    pub opaque_mask: Vec<u64>,      // 64*64 length
    pub transparent_mask: Vec<u64>, // 64*64 length

    // Voxel health
    health: HashMap<[Scalar; 3], i16>,

    // Changed over the last frame.
    changed: Vec<GridChange>,
}

#[derive(Debug, Component, Reflect)]
pub struct GridChange {
    pub point: [Scalar; 3],
    pub last_voxel: Voxel,
    pub new_voxel: Voxel,
}

impl VoxelChunk {
    pub fn new() -> Self {
        Self {
            voxels: vec![Voxel::Air.id(); unpadded::ARR_STRIDE],

            opaque_mask: vec![0u64; padded::SIZE * padded::SIZE],
            transparent_mask: vec![0u64; padded::SIZE * padded::SIZE],

            health: HashMap::default(),
            changed: Vec::new(),
        }
    }

    #[inline]
    pub fn linearize(&self, point: [Scalar; 3]) -> usize {
        unpadded::linearize(point)
    }

    #[inline]
    pub fn delinearize(&self, index: usize) -> [Scalar; 3] {
        unpadded::delinearize(index)
    }

    pub fn voxel_iter(&self) -> impl Iterator<Item = ([Scalar; 3], Voxel)> {
        (0..self.array_size()).map(|i| (self.delinearize(i), self.linear_voxel(i)))
    }

    pub fn get_voxel(&self, point: [Scalar; 3]) -> Option<Voxel> {
        if !self.in_chunk_bounds(point) {
            return None;
        }

        let index = self.linearize(point);
        Some(self.linear_voxel(index))
    }

    pub fn voxel(&self, point: [Scalar; 3]) -> Voxel {
        if !self.in_chunk_bounds(point) {
            panic!("Point out of bounds: {:?}", point);
        }

        let index = self.linearize(point);
        self.linear_voxel(index)
    }

    #[inline]
    pub fn linear_voxel(&self, index: usize) -> Voxel {
        Voxel::from_id(self.voxels[index]).unwrap()
    }

    pub fn set(&mut self, point: [Scalar; 3], voxel: Voxel) {
        if !self.in_chunk_bounds(point) {
            panic!("Point out of bounds: {:?}", point);
        }

        let index = self.linearize(point);
        let last_voxel = self.linear_voxel(index);
        if last_voxel != voxel {
            self.changed.push(GridChange { point, last_voxel, new_voxel: voxel });
        }

        self.clear_health(point);
        self.voxels[index as usize] = voxel.id();

        self.set_masks(point.map(|p| p + 1), voxel.transparent())
    }

    pub fn set_masks(&mut self, padded_point: [Scalar; 3], transparent: bool) {
        let padded_index = padded::linearize(padded_point);

        let mask_index = padded_index / padded::SIZE;
        let mask_bit = padded_index % padded::SIZE;
        let mask = 1 << mask_bit;

        if transparent {
            self.transparent_mask[mask_index] |= mask;
            self.opaque_mask[mask_index] &= !mask;
        } else {
            self.transparent_mask[mask_index] &= !mask;
            self.opaque_mask[mask_index] |= mask;
        }
    }

    pub fn changed(&self) -> impl Iterator<Item = &GridChange> {
        self.changed.iter()
    }

    pub fn clear_changed(&mut self) {
        self.changed.clear();
    }

    pub fn clear_changed_system(mut grids: Query<&mut Self>) {
        for mut grid in &mut grids {
            grid.clear_changed();
        }
    }

    /// Iterate over all points in this grid.
    pub fn point_iter(&self) -> impl Iterator<Item = [Scalar; 3]> {
        (0..self.array_size()).map(|i| self.delinearize(i))
    }

    #[inline]
    pub fn array_size(&self) -> usize {
        unpadded::ARR_STRIDE
    }

    #[inline]
    pub fn x_size(&self) -> Scalar {
        unpadded::SIZE as Scalar
    }

    #[inline]
    pub fn y_size(&self) -> Scalar {
        unpadded::SIZE as Scalar
    }

    #[inline]
    pub fn z_size(&self) -> Scalar {
        unpadded::SIZE as Scalar
    }

    /// Is this point within the bounds of this grid?
    #[inline]
    pub fn in_chunk_bounds(&self, point: [Scalar; 3]) -> bool {
        point[0] >= 0
            && point[1] >= 0
            && point[2] >= 0
            && point[0] < self.x_size()
            && point[1] < self.y_size()
            && point[2] < self.z_size()
    }

    #[inline]
    pub fn chunk_bounds(&self) -> [Scalar; 3] {
        [self.x_size(), self.y_size(), self.z_size()]
    }

    #[inline]
    pub fn world_bounds(&self) -> Vec3 {
        self.unscaled_world_bounds() * crate::voxel::GRID_SCALE
    }

    #[inline]
    pub fn unscaled_world_bounds(&self) -> Vec3 {
        Vec3::new(self.x_size() as f32, self.y_size() as f32, self.z_size() as f32)
    }

    #[inline]
    pub fn ground_level(&self) -> Scalar {
        (self.y_size() as f32 / 2.0).ceil() as Scalar
    }

    pub fn health(&self, point: [Scalar; 3]) -> i16 {
        if let Some(health) = self.health.get(&point) {
            *health
        } else {
            self.voxel(point).starting_health()
        }
    }

    pub fn set_health(&mut self, point: [Scalar; 3], health: i16) {
        self.health.insert(point, health);
    }

    pub fn clear_health(&mut self, point: [Scalar; 3]) {
        self.health.remove(&point);
    }

    // Closest voxel to the surface at a specified x and z.
    // This is a hack compared to a real screenspace raycast.
    pub fn surface_voxel(&self, x: Scalar, z: Scalar) -> Option<(Voxel, Scalar)> {
        for y in (0..self.y_size()).rev() {
            let voxel = self.voxel([x, y, z]);
            if voxel != Voxel::Air {
                return Some((voxel, y));
            }
        }

        None
    }

    pub fn cast_ray(
        &self,
        grid_transform: &GlobalTransform,
        ray: Ray3d,
        length: f32,
        mut gizmos: Option<&mut Gizmos>,
    ) -> Option<Hit> {
        let inv_matrix = grid_transform.compute_matrix().inverse();
        let local_direction =
            Dir3::new(inv_matrix.transform_vector3(ray.direction.as_vec3())).unwrap();
        let local_origin = inv_matrix.transform_vector3(ray.origin);

        let local_ray = Ray3d { origin: local_origin, direction: local_direction };

        let volume =
            BoundingVolume3 { size: IVec3::new(self.x_size(), self.y_size(), self.z_size()) };
        for mut hit in volume.traverse_ray(local_ray, length) {
            let local_distance = hit.distance;
            let local_point = local_ray.origin + local_ray.direction * local_distance;
            let world_point = grid_transform.transform_point(local_point);
            let world_distance = world_point.distance(ray.origin);
            hit.distance = world_distance;

            if let Some(gizmos) = &mut gizmos {
                //gizmos.sphere(world_point, 0.01, Color::srgb(1.0, 0.0, 0.0));

                /*
                let voxel_pos = Vec3::new(hit.voxel.0 as f32, hit.voxel.1 as f32, hit.voxel.2 as f32);
                let world_pos = grid_transform.transform_point(voxel_pos + Vec3::splat(0.5) * GRID_SCALE);
                gizmos.axes(Transform { translation: world_pos, ..default() }, 0.2);
                */
            }

            let voxel = self.voxel(hit.voxel.into());
            if voxel.pickable() {
                if let Some(gizmos) = &mut gizmos {
                    gizmos.sphere(world_point, 0.01, Color::srgb(1.0, 0.0, 0.0));
                }
                return Some(hit);
            }
        }

        None
    }

    /// Cast a ray in the localspace of the voxel grid.
    pub fn cast_local_ray(&self, ray: Ray3d, length: f32, gizmos: Option<Gizmos>) -> Option<Hit> {
        let volume =
            BoundingVolume3 { size: IVec3::new(self.x_size(), self.y_size(), self.z_size()) };
        for hit in volume.traverse_ray(ray, length) {
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
    use binary_greedy_meshing as bgm;

    use super::*;
    use crate::voxel::mesh::binary_greedy::BinaryGreedyMeshing;

    #[test]
    pub fn create_chunk() {
        // if this fails, probably allocated too much to stack
        let chunk = VoxelChunk::new();
    }

    #[test]
    pub fn linearize() {
        let sanity = |point| unpadded::delinearize(unpadded::linearize(point));
        assert_eq!(sanity([1, 1, 1]), [1, 1, 1]);
        assert_eq!(sanity([61, 5, 38]), [61, 5, 38]);
        assert_eq!(sanity([0, 0, 0]), [0, 0, 0]);
    }

    #[test]
    pub fn pad_linearize() {
        let sanity = |point| padded::delinearize(padded::linearize(point));
        assert_eq!(sanity([1, 1, 1]), [1, 1, 1]);
        assert_eq!(sanity([63, 5, 38]), [63, 5, 38]);
        assert_eq!(sanity([0, 0, 0]), [0, 0, 0]);
    }

    #[test]
    pub fn in_chunk_bounds() {
        let chunk = VoxelChunk::new();
        assert!(chunk.in_chunk_bounds([0, 0, 0]));
        assert!(chunk.in_chunk_bounds([4, 4, 4]));

        assert!(!chunk.in_chunk_bounds([63, 63, 63]));

        assert!(!chunk.in_chunk_bounds([63, 0, 0]));
        assert!(!chunk.in_chunk_bounds([0, 63, 0]));
        assert!(!chunk.in_chunk_bounds([0, 0, 63]));

        assert!(!chunk.in_chunk_bounds([-1, 0, 0]));
        assert!(!chunk.in_chunk_bounds([0, -1, 0]));
        assert!(!chunk.in_chunk_bounds([0, 0, -1]));
    }

    #[test]
    pub fn masks() {
        let mut chunk = VoxelChunk::new();
        let points = [[0, 0, 0], [1, 0, 0], [5, 2, 1], [9, 60, 3]];
        for point in points {
            let padded_point = point.map(|p| p + 1);
            let mask_index = padded::linearize(padded_point) / padded::SIZE;
            let mask_bit = padded::linearize(padded_point) % padded::SIZE;

            chunk.set(point, Voxel::Dirt);
            println!(
                "{:?}:{:?} = [{:?}] [{:?}]",
                mask_index,
                mask_bit,
                chunk.opaque_mask[mask_index],
                1 << mask_bit
            );
            assert_eq!(chunk.opaque_mask[mask_index] & (1 << mask_bit), 1 << mask_bit);
            assert_eq!(chunk.transparent_mask[mask_index] & (1 << mask_bit), 0);

            chunk.set(point, Voxel::Water);
            println!(
                "{:?}:{:?} = arr {:?} mask {:?}",
                mask_index,
                mask_bit,
                chunk.opaque_mask[mask_index],
                1 << mask_bit
            );
            assert_eq!(chunk.opaque_mask[mask_index] & (1 << mask_bit), 0);
            assert_eq!(chunk.transparent_mask[mask_index] & (1 << mask_bit), 1 << mask_bit);

            chunk.set(point, Voxel::Dirt);
            println!(
                "{:?}:{:?} = [{:?}] [{:?}]",
                mask_index,
                mask_bit,
                chunk.opaque_mask[mask_index],
                1 << mask_bit
            );
            assert_eq!(chunk.opaque_mask[mask_index] & (1 << mask_bit), 1 << mask_bit);
            assert_eq!(chunk.transparent_mask[mask_index] & (1 << mask_bit), 0);
        }

        assert_eq!(chunk.transparent_mask.iter().sum::<u64>(), 0);

        use std::collections::BTreeSet;
        let mut buffer = vec![0u16; bgm::CS_P3];
        let transparents =
            Voxel::iter().filter(|v| v.transparent()).map(|v| v.id()).collect::<BTreeSet<_>>();
        chunk.as_binary_voxels(&mut buffer);
        let mask = bgm::compute_opaque_mask(&buffer, &transparents);
        for index in 0..bgm::CS_P2 {
            assert_eq!(chunk.opaque_mask[index], mask[index]);
        }
    }
}
