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
}

impl Voxels {
    pub fn new() -> Self {
        Self { chunks: HashMap::new() }
    }

    /// Given a voxel position, find the chunk it is in.
    pub fn find_chunk(point: IVec3) -> IVec3 {
        point.div_euclid(IVec3::splat(unpadded::SIZE as Scalar))
    }

    pub fn relative_point(point: IVec3) -> IVec3 {
        point.rem_euclid(IVec3::splat(unpadded::SIZE as Scalar))
    }

    pub fn set_voxel(&mut self, point: IVec3, voxel: Voxel) {
        let chunk = self.chunks.entry(Self::find_chunk(point)).or_default();
        let relative_point = Self::relative_point(point);
        chunk.set(relative_point.into(), voxel);
    }

    pub fn get_voxel(&self, point: IVec3) -> Option<Voxel> {
        if let Some(chunk) = self.chunks.get(&Self::find_chunk(point)) {
            chunk.get_voxel(Self::relative_point(point).into())
        } else {
            None
        }
    }

    pub fn set_health(&mut self, point: IVec3, health: i16) {
        if let Some(chunk) = self.chunks.get_mut(&Self::find_chunk(point)) {
            chunk.set_health(point.into(), health);
        }
    }

    pub fn health(&self, point: IVec3) -> Option<i16> {
        if let Some(chunk) = self.chunks.get(&Self::find_chunk(point)) {
            Some(chunk.health(Self::relative_point(point).into()))
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
            max = max.max(chunk_point + IVec3::splat(unpadded::SIZE as Scalar));
        }

        (min, max)
    }

    pub fn chunk_size(&self) -> IVec3 {
        let (min, max) = self.chunk_bounds();
        let size = min.abs().max(max.abs());
        size
    }

    pub fn cast_ray(
        &self,
        grid_transform: &GlobalTransform,
        ray: Ray3d,
        length: f32,
        mut gizmos: Option<&mut Gizmos>,
    ) -> Option<Hit> {
        let inv_matrix = grid_transform.compute_matrix().inverse();
        let Ok(local_direction) = Dir3::new(inv_matrix.transform_vector3(ray.direction.as_vec3()))
        else {
            return None;
        };
        let local_origin = inv_matrix.transform_vector3(ray.origin);

        let local_ray = Ray3d { origin: local_origin, direction: local_direction };

        let volume = BoundingVolume3 { size: self.chunk_size() };
        for mut hit in volume.traverse_ray(local_ray, length) {
            let local_distance = hit.distance;
            let local_point = local_ray.origin + local_ray.direction * local_distance;
            let world_point = grid_transform.transform_point(local_point);
            let world_distance = world_point.distance(ray.origin);
            hit.distance = world_distance;

            if let Some(_gizmos) = &mut gizmos {
                //gizmos.sphere(world_point, 0.01, Color::srgb(1.0, 0.0, 0.0));

                /*
                let voxel_pos = Vec3::new(hit.voxel.0 as f32, hit.voxel.1 as f32, hit.voxel.2 as f32);
                let world_pos = grid_transform.transform_point(voxel_pos + Vec3::splat(0.5) * GRID_SCALE);
                gizmos.axes(Transform { translation: world_pos, ..default() }, 0.2);
                */
            }

            if let Some(voxel) = self.get_voxel(hit.voxel.into()) {
                if voxel.pickable() {
                    if let Some(gizmos) = &mut gizmos {
                        gizmos.sphere(world_point, 0.01, Color::srgb(1.0, 0.0, 0.0));
                    }
                    return Some(hit);
                }
            }
        }

        None
    }

    pub fn chunk_iter(&self) -> impl Iterator<Item = (IVec3, &VoxelChunk)> {
        self.chunks.iter().map(|(p, c)| (*p, c))
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
        if !self.in_chunk_bounds(point) {
            return None;
        }

        Some(self.voxel_from_index(padded::pad_linearize(point)))
    }

    pub fn voxel(&self, point: [Scalar; 3]) -> Voxel {
        if !self.in_chunk_bounds(point) {
            panic!("Point out of bounds: {:?}", point);
        }

        self.voxel_from_index(padded::pad_linearize(point))
    }

    #[inline]
    pub fn voxel_from_index(&self, index: usize) -> Voxel {
        Voxel::from_id(self.voxels[index]).unwrap()
    }

    pub fn set(&mut self, point: [Scalar; 3], voxel: Voxel) {
        if !self.in_chunk_bounds(point) {
            panic!("Point out of bounds: {:?}", point);
        }
        let padded_point = point.map(|p| p + 1);
        self.set_unpadded(padded_point, voxel);
    }

    pub fn set_unpadded(&mut self, point: [Scalar; 3], voxel: Voxel) {
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

    /// Cast a ray in the localspace of the voxel grid.
    pub fn cast_local_ray(&self, ray: Ray3d, length: f32, _gizmos: Option<Gizmos>) -> Option<Hit> {
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
        println!("{:?}", Voxels::find_chunk(IVec3::new(62, 0, 0)));
    }
}
