use std::ops::RangeInclusive;

use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;

use super::raycast::Hit;
use crate::voxel::raycast::VoxelHit;
use crate::voxel::{Voxel, VoxelAabb};

pub type Scalar = i32;

pub mod unpadded {
    use super::Scalar;

    pub const SIZE: usize = 62;
    pub const SIZE_SCALAR: Scalar = SIZE as Scalar;
    pub const USIZE: usize = SIZE as usize;
    pub const Z_STRIDE: usize = 1;
    pub const X_STRIDE: usize = SIZE;
    pub const Y_STRIDE: usize = SIZE * SIZE;
    pub const ARR_STRIDE: usize = SIZE * SIZE * SIZE;

    // Padded linearize point into a 62^3 XZY array
    #[inline]
    pub fn linearize([x, y, z]: [Scalar; 3]) -> usize {
        z as usize + x as usize * X_STRIDE + y as usize * Y_STRIDE
    }

    #[inline]
    pub fn pad_linearize([x, y, z]: [Scalar; 3]) -> usize {
        (z + 1) as usize + (x + 1) as usize * X_STRIDE + (y + 1) as usize * Y_STRIDE
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
    pub const USIZE: usize = SIZE as usize;
    pub const Z_STRIDE: usize = 1;
    pub const X_STRIDE: usize = SIZE;
    pub const Y_STRIDE: usize = SIZE * SIZE;
    pub const ARR_STRIDE: usize = SIZE * SIZE * SIZE;

    // Padded linearize point into a 64^3 XZY array
    #[inline]
    pub fn linearize([x, y, z]: [Scalar; 3]) -> usize {
        z as usize + x as usize * X_STRIDE + y as usize * Y_STRIDE
    }

    #[inline]
    pub fn pad_linearize([x, y, z]: [Scalar; 3]) -> usize {
        (z + 1) as usize + (x + 1) as usize * X_STRIDE + (y + 1) as usize * Y_STRIDE
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

#[derive(Debug, Component)]
#[require(Name::new("Voxels"))]
pub struct Voxels {
    chunks: HashMap<IVec3, VoxelChunk>, // spatially hashed chunks because its easy
    changed_chunks: HashSet<IVec3>,
}

impl Voxels {
    pub fn new() -> Self {
        Self { chunks: HashMap::new(), changed_chunks: HashSet::new() }
    }

    /// Given a voxel position, find the chunk it is in.
    #[inline]
    pub fn find_chunk(point: IVec3) -> IVec3 {
        point.div_euclid(IVec3::splat(unpadded::SIZE as Scalar))
    }

    fn chunks_overlapping_voxel(voxel_pos: IVec3) -> Vec<IVec3> {
        let min_chunk = IVec3::new(
            ((voxel_pos.x - unpadded::SIZE_SCALAR) as f32 / unpadded::SIZE as f32).floor() as i32,
            ((voxel_pos.y - unpadded::SIZE_SCALAR) as f32 / unpadded::SIZE as f32).floor() as i32,
            ((voxel_pos.z - unpadded::SIZE_SCALAR) as f32 / unpadded::SIZE as f32).floor() as i32,
        );

        let max_chunk = IVec3::new(
            ((voxel_pos.x + 1) as f32 / unpadded::SIZE as f32).ceil() as i32,
            ((voxel_pos.y + 1) as f32 / unpadded::SIZE as f32).ceil() as i32,
            ((voxel_pos.z + 1) as f32 / unpadded::SIZE as f32).ceil() as i32,
        );

        let mut chunks = Vec::new();

        for x in min_chunk.x..=max_chunk.x {
            for y in min_chunk.y..=max_chunk.y {
                for z in min_chunk.z..=max_chunk.z {
                    chunks.push(IVec3::new(x, y, z));
                }
            }
        }

        chunks
    }

    #[inline]
    pub fn relative_points(point: IVec3) -> [Option<(IVec3, IVec3)>; 8] {
        let base = Voxels::find_chunk(point);

        let relative_point = Self::relative_point(base, point);

        // Need to search for 8 chunks based on the boundaries of the point and give the relative points for each

        todo!()
    }

    pub fn relative_point(chunk: IVec3, world_point: IVec3) -> IVec3 {
        let chunk_origin = chunk * unpadded::SIZE as Scalar;
        world_point - chunk_origin
    }

    pub fn relative_point_with_boundary(chunk: IVec3, world_point: IVec3) -> IVec3 {
        Self::relative_point(chunk, world_point) + IVec3::ONE
    }

    // pub fn relative_point(point: IVec3) -> IVec3 {
    //     point.rem_euclid(IVec3::splat(unpadded::SIZE as Scalar))
    // }

    pub fn relative_point_padded(point: IVec3) -> IVec3 {
        point.rem_euclid(IVec3::splat(unpadded::SIZE as Scalar)) + IVec3::ONE
    }

    pub fn set_voxel(&mut self, point: IVec3, voxel: Voxel) {
        for chunk_point in Self::chunks_overlapping_voxel(point) {
            let chunk = self.chunks.entry(chunk_point).or_default();
            let relative_point = Self::relative_point_with_boundary(chunk_point, point);
            if chunk.in_chunk_bounds_unpadded(relative_point) {
                self.changed_chunks.insert(chunk_point);
                chunk.set_unpadded(relative_point.into(), voxel);
            }
        }
    }

    pub fn get_voxel(&self, point: IVec3) -> Option<Voxel> {
        let chunk_point = Self::find_chunk(point);
        if let Some(chunk) = self.chunks.get(&chunk_point) {
            chunk.get_voxel(Self::relative_point(chunk_point, point).into())
        } else {
            None
        }
    }

    pub fn get_chunk(&self, point: IVec3) -> Option<&VoxelChunk> {
        self.chunks.get(&point)
    }

    pub fn set_health(&mut self, point: IVec3, health: i16) {
        if let Some(chunk) = self.chunks.get_mut(&Self::find_chunk(point)) {
            chunk.set_health(point.into(), health);
        }
    }

    pub fn health(&self, point: IVec3) -> Option<i16> {
        let chunk_point = Self::find_chunk(point);
        if let Some(chunk) = self.chunks.get(&chunk_point) {
            Some(chunk.health(Self::relative_point(chunk_point, point).into()))
        } else {
            None
        }
    }

    // [min, max]
    pub fn chunk_bounds(&self) -> (IVec3, IVec3) {
        let mut min = IVec3::MAX;
        let mut max = IVec3::MIN;

        if self.chunks.len() == 0 {
            return (IVec3::ZERO, IVec3::ZERO);
        }

        for chunk_point in self.chunks.keys().copied() {
            min = min.min(chunk_point);
            max = max.max(chunk_point + IVec3::splat(1));
        }

        (min, max)
    }

    pub fn voxel_bounds(&self) -> (IVec3, IVec3) {
        let (min, max) = self.chunk_bounds();
        (min * unpadded::SIZE as Scalar, max * unpadded::SIZE as Scalar)
    }

    pub fn chunk_size(&self) -> IVec3 {
        let (min, max) = self.chunk_bounds();
        max - min
    }

    /// Raycasting in chunk space.
    pub fn chunk_ray_iter(
        &self,
        grid_transform: &GlobalTransform,
        ray: Ray3d,
        length: f32,
    ) -> impl Iterator<Item = Hit> {
        const CHUNK_SIZE: Vec3 = Vec3::splat(unpadded::SIZE as f32);
        #[allow(non_snake_case)]
        let SCALED_CHUNK_SIZE: Vec3 = CHUNK_SIZE * crate::voxel::GRID_SCALE;

        let inv_matrix = grid_transform.compute_matrix().inverse();
        let Ok(local_direction) = Dir3::new(inv_matrix.transform_vector3(ray.direction.as_vec3()))
        else {
            panic!();
        };
        let chunk_scaled = Transform { scale: 1.0 / CHUNK_SIZE, ..default() };
        let local_origin =
            chunk_scaled.compute_matrix().transform_point3(inv_matrix.transform_point3(ray.origin));

        let local_ray = Ray3d { origin: local_origin, direction: local_direction };

        let (min, max) = self.chunk_bounds();
        let volume = VoxelAabb { min, max };
        // info!("chunk bounds: {:?}", self.chunk_size());chunk
        volume.traverse_ray(local_ray, length)
    }

    pub fn local_voxel_ray_iter(
        &self,
        chunk_transform: &GlobalTransform,
        chunk_pos: IVec3,
        ray: Ray3d,
        length: f32,
    ) -> impl Iterator<Item = VoxelHit> {
        // translate ray to voxel space
        let local_ray = {
            let inv_matrix = chunk_transform.compute_matrix().inverse();
            Ray3d {
                origin: inv_matrix.transform_point3(ray.origin),
                direction: Dir3::new(inv_matrix.transform_vector3(ray.direction.as_vec3()))
                    .unwrap(),
            }
        };

        let min = chunk_pos * unpadded::SIZE as i32;
        let max = min + IVec3::splat(unpadded::SIZE as i32);
        let volume = VoxelAabb { min, max };
        volume.traverse_ray(local_ray, length).into_iter().map(move |hit| {
            // translate hit back to world space
            let local_distance = hit.distance;
            let local_point = local_ray.origin + local_ray.direction.as_vec3() * local_distance;
            let world_point = chunk_transform.transform_point(local_point);
            let world_distance = world_point.distance(ray.origin);
            VoxelHit {
                chunk: chunk_pos,
                voxel: hit.voxel,
                world_space: world_point,

                distance: world_distance,
                normal: hit.normal,
            }
        })
    }

    /// Raycasting in chunk space.
    pub fn ray_iter(
        &self,
        grid_transform: &GlobalTransform,
        ray: Ray3d,
        length: f32,
    ) -> impl Iterator<Item = VoxelHit> {
        self.chunk_ray_iter(grid_transform, ray, length).flat_map(move |chunk_hit| {
            const CHUNK_SIZE: Vec3 = Vec3::splat(unpadded::SIZE as f32);

            // let chunk_pos = chunk_hit.voxel.as_vec3() * CHUNK_SIZE;
            // let chunk_pos_transform = Transform { translation: chunk_pos, ..default() };
            // let chunk_transform = grid_transform.mul_transform(chunk_pos_transform);

            self.local_voxel_ray_iter(grid_transform, chunk_hit.voxel, ray, length)
        })
    }

    pub fn cast_ray(
        &self,
        grid_transform: &GlobalTransform,
        ray: Ray3d,
        length: f32,
    ) -> Option<VoxelHit> {
        for hit in self.ray_iter(grid_transform, ray, length) {
            // info!("hit: {hit:?}");
            if let Some(voxel) = self.get_voxel(hit.voxel) {
                if voxel.pickable() {
                    return Some(hit);
                }
            }
        }
        None
    }

    pub fn chunk_iter(&self) -> impl Iterator<Item = (IVec3, &VoxelChunk)> {
        self.chunks.iter().map(|(p, c)| (*p, c))
    }

    pub fn changed_chunk_pos_iter(&self) -> impl Iterator<Item = IVec3> {
        self.changed_chunks.iter().copied()
    }

    pub fn changed_chunk_iter(&self) -> impl Iterator<Item = (IVec3, &VoxelChunk)> {
        self.changed_chunks.iter().filter_map(|p| self.chunks.get(p).map(|chunk| (*p, chunk)))
    }

    pub fn clear_changed_chunks(&mut self) {
        self.changed_chunks.clear();
    }

    /// Take a 3x3 matrix around the voxel and smooth it on the y-axis.
    pub fn smooth_voxel(&mut self, pos: IVec3, height_range: RangeInclusive<i32>) {
        #[rustfmt::skip]
        let surrounding = [
            IVec3::new(-1, 0, 1), IVec3::new(0, 0, 1), IVec3::new(1, 0, 1),
            IVec3::new(-1, 0, 0), IVec3::new(0, 0, 0), IVec3::new(1, 0, 0),
            IVec3::new(-1, 0, -1), IVec3::new(0, 0, -1), IVec3::new(1, 0, -1),
        ];

        #[rustfmt::skip]
        let convolution = [
            1, 2, 1,
            2, 3, 2,
            1, 2, 1,
        ];

        let mut heights = [0; 9];
        for (index, relative) in surrounding.iter().enumerate() {
            // find the height by searching up and down until we find [`Voxel::Air`].
            let voxel = pos + relative;

            match self.get_voxel(voxel) {
                Some(Voxel::Air) | None => {
                    while heights[index] > *height_range.start() {
                        match self.get_voxel(voxel + IVec3::Y * heights[index]) {
                            Some(Voxel::Air) | None => {},
                            _ => break,
                        }

                        heights[index] -= 1;
                    }
                },
                _ => {
                    while heights[index] < *height_range.end() {
                        match self.get_voxel(voxel + IVec3::Y * heights[index]) {
                            Some(Voxel::Air) | None => break,
                            _ => {},
                        }

                        heights[index] += 1;
                    }
                },
            };
        }

        let mut sum = 0;
        for index in 0..9 {
            sum += heights[index] * convolution[index];
        }
        let new_height = sum / 9;

        // create a gradient over the range to the current height or the new height if
        // lower.
        let mut gradient = Vec::new();
        for height in *height_range.start()..new_height.max(0) {
            let voxel = match self.get_voxel(pos + IVec3::Y * height) {
                Some(voxel) => voxel,
                None => Voxel::Air,
            };

            gradient.push(voxel);
        }

        if new_height < pos.y {
            // squish the voxels down
        } else {
            // stretch the voxels out
        }
    }
}

/// Single voxel chunk, 64^3 (1 padding on the edges for meshing)
#[derive(Debug, Component)]
#[require(Name::new("Voxel Chunk"))]
pub struct VoxelChunk {
    pub voxels: Vec<u16>,           // padded::ARR_STRIDE length
    pub opaque_mask: Vec<u64>,      // padded::SIZE^2 length, bit masks of 64^3 voxels
    pub transparent_mask: Vec<u64>, // padded::SIZE^2 length

    // Voxel health
    health: HashMap<[Scalar; 3], i16>,
}

impl Default for VoxelChunk {
    fn default() -> Self {
        Self {
            voxels: vec![Voxel::Air.id(); padded::ARR_STRIDE],

            opaque_mask: vec![0u64; padded::SIZE * padded::SIZE],
            transparent_mask: vec![0u64; padded::SIZE * padded::SIZE],

            health: HashMap::default(),
        }
    }
}

impl VoxelChunk {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_voxel(&self, point: [Scalar; 3]) -> Option<Voxel> {
        if !self.in_chunk_bounds(point.into()) {
            return None;
        }

        Some(self.voxel_from_index(padded::pad_linearize(point)))
    }

    pub fn voxel(&self, point: [Scalar; 3]) -> Voxel {
        if !self.in_chunk_bounds(point.into()) {
            panic!("Point out of bounds: {:?}", point);
        }

        self.voxel_from_index(padded::pad_linearize(point))
    }

    #[inline]
    pub fn voxel_from_index(&self, index: usize) -> Voxel {
        Voxel::from_id(self.voxels[index]).unwrap()
    }

    pub fn set(&mut self, point: [Scalar; 3], voxel: Voxel) {
        if !self.in_chunk_bounds(point.into()) {
            panic!("Point out of bounds: {:?}", point);
        }
        let padded_point = point.map(|p| p + 1);
        self.set_unpadded(padded_point, voxel);
    }

    pub fn set_unpadded(&mut self, point: [Scalar; 3], voxel: Voxel) {
        if !self.in_chunk_bounds_unpadded(point.into()) {
            panic!("Point out of bounds: {:?}", point);
        }

        let index = padded::linearize(point);

        self.clear_health(point);
        self.voxels[index as usize] = voxel.id();
        self.set_masks(point, voxel.transparent())
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

    pub fn voxel_iter(&self) -> impl Iterator<Item = ([Scalar; 3], Voxel)> {
        self.point_iter().map(|p| (p, self.voxel(p)))
    }

    /// Iterate over all points in this grid.
    pub fn point_iter(&self) -> impl Iterator<Item = [Scalar; 3]> {
        // iterate x -> z -> y, same as stored because cache
        struct PointIter {
            current: [Scalar; 3],
            done: bool,
        }
        impl Iterator for PointIter {
            type Item = [Scalar; 3];

            fn next(&mut self) -> Option<Self::Item> {
                if self.done {
                    None
                } else {
                    let next = self.current;
                    self.current[0] += 1;
                    if self.current[0] >= unpadded::SIZE as Scalar {
                        self.current[0] = 0;
                        self.current[2] += 1;
                        if self.current[2] >= unpadded::SIZE as Scalar {
                            self.current[2] = 0;
                            self.current[1] += 1;
                            if self.current[1] >= unpadded::SIZE as Scalar {
                                self.done = true;
                            }
                        }
                    }

                    Some(next)
                }
            }
        }

        PointIter { current: [0; 3], done: false }
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
    pub fn in_chunk_bounds(&self, point: IVec3) -> bool {
        point.x >= 0
            && point.y >= 0
            && point.z >= 0
            && point.x < self.x_size()
            && point.y < self.y_size()
            && point.z < self.z_size()
    }

    #[inline]
    pub fn in_chunk_bounds_unpadded(&self, point: IVec3) -> bool {
        point.x >= 0
            && point.y >= 0
            && point.z >= 0
            && point.x < padded::SIZE as Scalar
            && point.y < padded::SIZE as Scalar
            && point.z < padded::SIZE as Scalar
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

    #[test]
    pub fn create_chunk() {
        // if this fails, probably allocated too much to stack
        VoxelChunk::new();
    }

    #[test]
    pub fn set_chunk_voxel() {
        let mut chunk = VoxelChunk::new();
        chunk.set([0, 0, 0], Voxel::Dirt);
        assert_eq!(chunk.voxel([0, 0, 0]), Voxel::Dirt);
        chunk.set([0, 0, 0], Voxel::Water);
        assert_eq!(chunk.voxel([0, 0, 0]), Voxel::Water);
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
        assert!(chunk.in_chunk_bounds(ivec3(0, 0, 0)));
        assert!(chunk.in_chunk_bounds(ivec3(4, 4, 4)));

        assert!(!chunk.in_chunk_bounds(ivec3(62, 62, 62)));
        assert!(chunk.in_chunk_bounds(ivec3(61, 61, 61)));

        assert!(!chunk.in_chunk_bounds(ivec3(62, 0, 0)));
        assert!(!chunk.in_chunk_bounds(ivec3(0, 62, 0)));
        assert!(!chunk.in_chunk_bounds(ivec3(0, 0, 62)));

        assert!(chunk.in_chunk_bounds(ivec3(61, 0, 0)));
        assert!(chunk.in_chunk_bounds(ivec3(0, 61, 0)));
        assert!(chunk.in_chunk_bounds(ivec3(0, 0, 61)));

        assert!(!chunk.in_chunk_bounds(ivec3(-1, 0, 0)));
        assert!(!chunk.in_chunk_bounds(ivec3(0, -1, 0)));
        assert!(!chunk.in_chunk_bounds(ivec3(0, 0, -1)));
    }

    #[test]
    pub fn point_iter() {
        let chunk = VoxelChunk::new();
        let mut iter = chunk.point_iter();
        assert_eq!(iter.next(), Some([0, 0, 0]));
        assert_eq!(iter.next(), Some([1, 0, 0]));
        for _ in 0..60 {
            iter.next();
        }
        assert_eq!(iter.next(), Some([0, 0, 1]));
        assert_eq!(iter.last(), Some([unpadded::SIZE as Scalar - 1; 3]));
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

        for [x, y, z] in points {
            println!(
                "{:?} ? {:?}",
                bgm::pad_linearize(x as usize, y as usize, z as usize),
                padded::pad_linearize([x, y, z])
            );
        }

        use std::collections::BTreeSet;
        let transparents =
            Voxel::iter().filter(|v| v.transparent()).map(|v| v.id()).collect::<BTreeSet<_>>();
        let mask = bgm::compute_opaque_mask(&chunk.voxels, &transparents);
        for index in 0..bgm::CS_P2 {
            assert_eq!(chunk.opaque_mask[index], mask[index]);
        }
    }

    #[test]
    fn find_chunk() {
        assert_eq!(Voxels::find_chunk(ivec3(0, 0, 0)), ivec3(0, 0, 0));
        assert_eq!(Voxels::find_chunk(ivec3(63, 0, 0)), ivec3(1, 0, 0));
        assert_eq!(Voxels::find_chunk(ivec3(-1, 0, 0)), ivec3(-1, 0, 0));
        assert_eq!(Voxels::find_chunk(ivec3(-62, 0, 0)), ivec3(-1, 0, 0));
        assert_eq!(Voxels::find_chunk(ivec3(-63, 0, 0)), ivec3(-2, 0, 0));
    }

    #[test]
    fn find_chunk_relative() {
        assert_eq!(Voxels::relative_point(ivec3(0, 0, 0), ivec3(0, 0, 0)), ivec3(0, 0, 0));
        assert_eq!(Voxels::relative_point(ivec3(0, 0, 0), ivec3(61, 0, 0)), ivec3(61, 0, 0));
        assert_eq!(Voxels::relative_point(ivec3(0, 0, 0), ivec3(62, 0, 0)), ivec3(62, 0, 0)); // oob
        assert_eq!(Voxels::relative_point(ivec3(0, 0, 0), ivec3(63, 0, 0)), ivec3(63, 0, 0)); // oob
        assert_eq!(Voxels::relative_point(ivec3(1, 0, 0), ivec3(62, 0, 0)), ivec3(0, 0, 0));
        assert_eq!(Voxels::relative_point(ivec3(1, 0, 0), ivec3(63, 0, 0)), ivec3(1, 0, 0));

        // negative handling
        assert_eq!(Voxels::relative_point(ivec3(0, 0, 0), ivec3(-1, -1, -1)), ivec3(-1, -1, -1));
        assert_eq!(Voxels::relative_point(ivec3(-1, -1, -1), ivec3(0, 0, 0)), ivec3(62, 62, 62)); // oob
        assert_eq!(Voxels::relative_point(ivec3(-1, -1, -1), ivec3(-1, -1, -1)), ivec3(61, 61, 61));
        assert_eq!(Voxels::relative_point(ivec3(-1, -1, -1), ivec3(-62, -62, -62)), ivec3(0, 0, 0));
    }

    #[test]
    fn find_chunk_relative_unpadded() {
        assert_eq!(
            Voxels::relative_point_with_boundary(ivec3(0, 0, 0), ivec3(0, 0, 0)),
            ivec3(1, 1, 1)
        );
        assert_eq!(
            Voxels::relative_point_with_boundary(ivec3(0, 0, 0), ivec3(62, 0, 0)),
            ivec3(63, 1, 1)
        ); // oob
        assert_eq!(
            Voxels::relative_point_with_boundary(ivec3(0, 0, 0), ivec3(63, 0, 0)),
            ivec3(64, 1, 1)
        ); // oob
        assert_eq!(
            Voxels::relative_point_with_boundary(ivec3(1, 0, 0), ivec3(61, 0, 0)),
            ivec3(0, 1, 1)
        );
        assert_eq!(
            Voxels::relative_point_with_boundary(ivec3(1, 0, 0), ivec3(62, 0, 0)),
            ivec3(1, 1, 1)
        );

        assert_eq!(
            Voxels::relative_point_with_boundary(ivec3(0, 0, 0), ivec3(61, 61, 61)),
            ivec3(62, 62, 62)
        );
        assert_eq!(
            Voxels::relative_point_with_boundary(ivec3(1, 1, 1), ivec3(61, 61, 61)),
            ivec3(0, 0, 0)
        );

        // negative handling
        assert_eq!(
            Voxels::relative_point_with_boundary(ivec3(0, 0, 0), ivec3(-1, -1, -1)),
            ivec3(0, 0, 0)
        );
        assert_eq!(
            Voxels::relative_point_with_boundary(ivec3(-1, -1, -1), ivec3(0, 0, 0)),
            ivec3(63, 63, 63)
        );
        assert_eq!(
            Voxels::relative_point_with_boundary(ivec3(-1, -1, -1), ivec3(-1, -1, -1)),
            ivec3(62, 62, 62)
        );
        assert_eq!(
            Voxels::relative_point_with_boundary(ivec3(-1, -1, -1), ivec3(-62, -62, -62)),
            ivec3(1, 1, 1)
        );
        assert_eq!(
            Voxels::relative_point_with_boundary(ivec3(-1, -1, -1), ivec3(-63, -63, -63)),
            ivec3(0, 0, 0)
        );
    }
}
