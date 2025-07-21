use std::cmp::Ordering;
use std::ops::RangeInclusive;

use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;

use super::raycast::Hit;
use crate::voxel::raycast::VoxelHit;
use crate::voxel::{Scalar, UpdateVoxelMeshSet, Voxel, VoxelAabb, VoxelChunk, padded, unpadded};

#[cfg(feature = "trace")]
use tracing::*;

pub fn plugin(app: &mut App) {
    app.add_event::<ChangedChunks>();

    // app.add_plugins(super::voxel::plugin);

    app.add_systems(PostUpdate, clear_changed_chunks.before(UpdateVoxelMeshSet));
}

#[derive(Event, Debug)]
pub struct ChangedChunks {
    pub voxel_entity: Entity,
    pub changed_chunks: Vec<IVec3>,
}

pub fn clear_changed_chunks(
    mut voxels: Query<(Entity, &mut Voxels)>,
    mut writer: EventWriter<ChangedChunks>,
) {
    for (voxel_entity, mut voxels) in &mut voxels {
        writer.write(ChangedChunks {
            voxel_entity,
            changed_chunks: voxels.changed_chunk_pos_iter().collect::<Vec<_>>(),
        });
        voxels.clear_changed_chunks();
    }
}

#[derive(Deref, DerefMut, Debug, Clone, PartialEq, Eq)]
pub struct VoxelUpdate(pub IVec3);

impl PartialOrd for VoxelUpdate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VoxelUpdate {
    fn cmp(&self, other: &Self) -> Ordering {
        other.y.cmp(&self.y).then(other.x.cmp(&self.x)).then(other.z.cmp(&self.z))
    }
}

#[derive(Debug, Component, Clone, PartialEq, Eq)]
#[require(Name::new("Voxels"))]
pub struct Voxels {
    chunks: HashMap<IVec3, VoxelChunk>, // spatially hashed chunks because its easy
    changed_chunks: HashSet<IVec3>,
    pub(crate) update_voxels: Vec<VoxelUpdate>,
    clip: VoxelAabb,
}

const CHUNK_SIZE: IVec3 = IVec3::splat(unpadded::SIZE as Scalar);
const CHUNK_SIZE_FLOAT: Vec3 = Vec3::splat(unpadded::SIZE as f32);

impl Voxels {
    pub fn new() -> Self {
        Self {
            chunks: default(),
            changed_chunks: default(),
            update_voxels: default(),
            clip: VoxelAabb {
                min: IVec3::new(-1000, -100, -1000),
                max: IVec3::new(1000, 300, 1000),
            },
        }
    }

    /// Given a voxel position, find the chunk it is in.
    #[inline]
    pub fn find_chunk(point: IVec3) -> IVec3 {
        #[cfg(feature = "trace")]
        let find_chunk_span = info_span!("find_chunk");

        point.div_euclid(CHUNK_SIZE)
    }

    #[inline]
    pub fn chunks_overlapping_voxel(voxel_pos: IVec3) -> impl Iterator<Item = IVec3> {
        #[cfg(feature = "trace")]
        let chunks_overlapping_voxel_span = info_span!("chunks_overlapping_voxel");

        let min_chunk =
            ((voxel_pos - IVec3::splat(2)).as_vec3() / CHUNK_SIZE_FLOAT).floor().as_ivec3();
        let max_chunk =
            ((voxel_pos + IVec3::splat(2)).as_vec3() / CHUNK_SIZE_FLOAT).ceil().as_ivec3();

        (min_chunk.y..max_chunk.y).flat_map(move |y| {
            (min_chunk.x..max_chunk.x)
                .flat_map(move |x| (min_chunk.z..max_chunk.z).map(move |z| IVec3::new(x, y, z)))
        })
    }

    #[inline]
    pub fn relative_point(chunk: IVec3, world_point: IVec3) -> IVec3 {
        let chunk_origin = chunk * unpadded::SIZE as Scalar;
        world_point - chunk_origin
    }

    #[inline]
    pub fn relative_point_with_boundary(chunk: IVec3, world_point: IVec3) -> IVec3 {
        Self::relative_point(chunk, world_point) + IVec3::ONE
    }

    // pub fn relative_point(point: IVec3) -> IVec3 {
    //     point.rem_euclid(IVec3::splat(unpadded::SIZE as Scalar))
    // }

    #[inline]
    pub fn relative_point_unoriented(point: IVec3) -> IVec3 {
        point.rem_euclid(CHUNK_SIZE)
    }

    #[inline]
    pub fn is_boundary_point(point: IVec3) -> bool {
        let relative_point = point.rem_euclid(CHUNK_SIZE);
        relative_point.x == 0
            || relative_point.x == unpadded::SIZE_SCALAR
            || relative_point.y == 0
            || relative_point.y == unpadded::SIZE_SCALAR
            || relative_point.z == 0
            || relative_point.z == unpadded::SIZE_SCALAR
    }

    pub fn get_relative_points(
        points: impl Iterator<Item = IVec3>,
    ) -> impl Iterator<Item = (IVec3, IVec3)> {
        points.flat_map(|point| {
            Self::chunks_overlapping_voxel(point).map(move |chunk_point| {
                (chunk_point, Self::relative_point_with_boundary(chunk_point, point))
            })
        })
    }

    #[inline]
    pub fn set_voxel_chunk_overlap(&mut self, point: IVec3, voxel: Voxel) {
        #[cfg(feature = "trace")]
        let set_voxel_overlap_span = info_span!("set_voxel_overlap_loop");

        for chunk_point in Self::chunks_overlapping_voxel(point) {
            #[cfg(feature = "trace")]
            let set_voxel_single_chunk_span = info_span!("set_voxel_single_chunk");

            let chunk = self.chunks.entry(chunk_point).or_default();
            let relative_point = Self::relative_point_with_boundary(chunk_point, point);
            if chunk.in_chunk_bounds_unpadded(relative_point) {
                self.changed_chunks.insert(chunk_point); // negligible
                chunk.set_unpadded(relative_point.into(), voxel);
            }
        }
    }

    #[inline]
    pub fn set_voxel(&mut self, point: IVec3, voxel: Voxel) {
        #[cfg(feature = "trace")]
        let set_voxel_span = info_span!("set_voxel");

        // if !self.clip.contains_point(point) {
        //     warn!("attempted voxel set at clip boundary");
        //     return;
        // }

        if point.y < -50 || point.y > 250 {
            return;
        }

        // if !Voxels::is_boundary_point(point) {
        //     #[cfg(feature = "trace")]
        //     let set_voxel_nonboundary_span = info_span!("set_voxel_nonboundary");

        //     let chunk_point = Self::find_chunk(point);
        //     let chunk = self.chunks.entry(chunk_point).or_default();
        //     let relative_point = Self::relative_point_unoriented(point);
        //     chunk.set(relative_point.into(), voxel);
        //     self.changed_chunks.insert(chunk_point); // negligible
        // } else {
        // Set the overlapping chunks boundary voxels as well
        // setting overlap chunks adds about 10% to the simulation time
        // }

        self.set_voxel_chunk_overlap(point, voxel);
        self.set_update_voxels(point); // 18-22%
    }

    pub fn set_voxels(
        &mut self,
        voxel_points: impl Iterator<Item = IVec3> + Clone,
        voxels: impl Iterator<Item = Voxel>,
    ) {
        for point in voxel_points.clone() {
            self.set_update_voxels(point);
        }

        let iterator = voxel_points.zip(voxels).flat_map(|(point, voxel)| {
            Self::chunks_overlapping_voxel(point).map(move |chunk_point| {
                (chunk_point, Self::relative_point_with_boundary(chunk_point, point), voxel)
            })
        });

        for (chunk_point, relative_point, voxel) in iterator {
            let chunk = self.chunks.entry(chunk_point).or_default();
            if chunk.in_chunk_bounds_unpadded(relative_point) {
                self.changed_chunks.insert(chunk_point);
                chunk.set_unpadded(relative_point.into(), voxel);
            }
        }
    }

    // Push point and adjacent 26 points into voxels to check simulation on.
    #[inline]
    pub fn set_update_voxels(&mut self, point: IVec3) {
        self.update_voxels.extend((-1..=1).flat_map(move |y| {
            (-1..=1)
                .flat_map(move |x| (-1..=1).map(move |z| VoxelUpdate(point + IVec3::new(x, y, z))))
        }));
    }

    #[inline]
    pub fn add_update_voxel(&mut self, point: IVec3) {
        self.update_voxels.push(VoxelUpdate(point));
    }

    pub fn get_voxel(&self, point: IVec3) -> Voxel {
        #[cfg(feature = "trace")]
        let get_voxel_span = info_span!("get_voxel");

        let chunk_point = Self::find_chunk(point);
        if let Some(chunk) = self.chunks.get(&chunk_point) {
            chunk.voxel(Self::relative_point(chunk_point, point))
        } else {
            Voxel::Air
        }
    }

    #[inline]
    pub fn get_nearby_voxels<const N: usize>(
        &self,
        point: IVec3,
        offsets: [[Scalar; 3]; N],
    ) -> [Voxel; N] {
        let center = Self::relative_point_unoriented(point);
        if center.x == 0
            || center.x == unpadded::SIZE_SCALAR
            || center.y == 0
            || center.y == unpadded::SIZE_SCALAR
            || center.z == 0
            || center.z == unpadded::SIZE_SCALAR
        {
            offsets.map(|offset| self.get_voxel(point + IVec3::from(offset)))
        } else {
            // assume we are in the same chunk
            let chunk_point = Self::find_chunk(point);
            let Some(chunk) = self.get_chunk(chunk_point) else {
                return [Voxel::Air; N];
            };

            let center_index = padded::pad_linearize(center.into());

            let strides = offsets.map(|offset| padded::pad_linearize_offset(offset));
            strides.map(|stride| chunk.voxel_from_index(center_index + stride as usize))
        }
    }

    // greedily find adjacent voxels by:
    // 1. get the relative position for the center position
    // 2. check if its on a border
    // 3. if not we are good to go.
    // 4. otherwise fall back to `get_voxel`
    pub fn get_adjacent_voxels(&self, point: IVec3) -> [Voxel; 27] {
        let center = Self::relative_point_unoriented(point);
        if center.x == 0
            || center.x == unpadded::SIZE_SCALAR
            || center.y == 0
            || center.y == unpadded::SIZE_SCALAR
            || center.z == 0
            || center.z == unpadded::SIZE_SCALAR
        {
            // fallback to `get_voxel`
            // IVec offsets
            const ADJACENCY_IVECS: [IVec3; 27] = [
                // 3x3 below
                IVec3::new(-1, -1, -1),
                IVec3::new(-1, -1, 0),
                IVec3::new(-1, -1, 1),
                IVec3::new(0, -1, -1),
                IVec3::new(0, -1, 0),
                IVec3::new(0, -1, 1),
                IVec3::new(1, -1, -1),
                IVec3::new(1, -1, 0),
                IVec3::new(1, -1, 1),
                // 3x3 sandwiched/center
                IVec3::new(-1, 0, -1),
                IVec3::new(-1, 0, 0),
                IVec3::new(-1, 0, 1),
                IVec3::new(0, 0, -1),
                IVec3::new(0, 0, 0),
                IVec3::new(0, 0, 1),
                IVec3::new(1, 0, -1),
                IVec3::new(1, 0, 0),
                IVec3::new(1, 0, 1),
                // 3x3 above
                IVec3::new(-1, 1, -1),
                IVec3::new(-1, 1, 0),
                IVec3::new(-1, 1, 1),
                IVec3::new(0, 1, -1),
                IVec3::new(0, 1, 0),
                IVec3::new(0, 1, 1),
                IVec3::new(1, 1, -1),
                IVec3::new(1, 1, 0),
                IVec3::new(1, 1, 1),
            ];

            ADJACENCY_IVECS.map(|offset| self.get_voxel(point + offset))
        } else {
            // assume we are in the same chunk
            let chunk_point = Self::find_chunk(point);
            let Some(chunk) = self.get_chunk(chunk_point) else {
                return [Voxel::Air; 27];
            };

            let center_index = padded::pad_linearize(center.into());

            // direct offsets:
            const ADJACENCY_STRIDES: [isize; 27] = [
                // 3x3 above
                -padded::Y_STRIDE_I - padded::X_STRIDE_I - padded::Z_STRIDE_I,
                -padded::Y_STRIDE_I - padded::X_STRIDE_I,
                -padded::Y_STRIDE_I - padded::X_STRIDE_I + padded::Z_STRIDE_I,
                -padded::Y_STRIDE_I - padded::Z_STRIDE_I,
                -padded::Y_STRIDE_I,
                -padded::Y_STRIDE_I + padded::Z_STRIDE_I,
                -padded::Y_STRIDE_I + padded::X_STRIDE_I - padded::Z_STRIDE_I,
                -padded::Y_STRIDE_I + padded::X_STRIDE_I,
                -padded::Y_STRIDE_I + padded::X_STRIDE_I + padded::Z_STRIDE_I,
                // 3x3 sandwiched/center
                padded::X_STRIDE_I - padded::Z_STRIDE_I,
                padded::X_STRIDE_I,
                padded::X_STRIDE_I + padded::Z_STRIDE_I,
                padded::Z_STRIDE_I,
                0,
                padded::Z_STRIDE_I,
                padded::X_STRIDE_I - padded::Z_STRIDE_I,
                padded::X_STRIDE_I,
                padded::X_STRIDE_I + padded::Z_STRIDE_I,
                // 3x3 below
                padded::Y_STRIDE_I - padded::X_STRIDE_I - padded::Z_STRIDE_I,
                padded::Y_STRIDE_I - padded::X_STRIDE_I,
                padded::Y_STRIDE_I - padded::X_STRIDE_I + padded::Z_STRIDE_I,
                padded::Y_STRIDE_I - padded::Z_STRIDE_I,
                padded::Y_STRIDE_I,
                padded::Y_STRIDE_I + padded::Z_STRIDE_I,
                padded::Y_STRIDE_I + padded::X_STRIDE_I - padded::Z_STRIDE_I,
                padded::Y_STRIDE_I + padded::X_STRIDE_I,
                padded::Y_STRIDE_I + padded::X_STRIDE_I + padded::Z_STRIDE_I,
            ];

            ADJACENCY_STRIDES.map(|stride| chunk.voxel_from_index(center_index + stride as usize))
        }
    }

    pub fn get_chunk(&self, point: IVec3) -> Option<&VoxelChunk> {
        self.chunks.get(&point)
    }

    pub fn get_chunk_mut(&mut self, point: IVec3) -> Option<&mut VoxelChunk> {
        self.chunks.get_mut(&point)
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

    pub fn chunk_aabb(&self) -> VoxelAabb {
        let (min, max) = self.chunk_bounds();
        VoxelAabb::new(min, max)
    }

    pub fn voxel_bounds(&self) -> (IVec3, IVec3) {
        let (min, max) = self.chunk_bounds();
        (min * unpadded::SIZE as Scalar, max * unpadded::SIZE as Scalar)
    }

    pub fn voxel_aabb(&self) -> VoxelAabb {
        let (min, max) = self.voxel_bounds();
        VoxelAabb::new(min, max)
    }

    pub fn chunk_size(&self) -> IVec3 {
        let (min, max) = self.chunk_bounds();
        max - min
    }

    pub fn chunk_pos_iter(&self) -> impl Iterator<Item = IVec3> {
        self.chunks.keys().copied()
    }

    pub fn point_iter(&self) -> impl Iterator<Item = IVec3> {
        self.chunk_pos_iter().flat_map(move |chunk_point| {
            let chunk_base = chunk_point * unpadded::SIZE as Scalar;
            VoxelChunk::point_iter().map(move |point| chunk_base + IVec3::from(point))
        })
    }

    /// Raycasting in chunk space.
    pub fn chunk_ray_iter(
        &self,
        grid_transform: &GlobalTransform,
        ray: Ray3d,
        length: f32,
    ) -> impl Iterator<Item = Hit> {
        const CHUNK_SIZE: Vec3 = Vec3::splat(unpadded::SIZE as f32);
        // #[allow(non_snake_case)]
        // let SCALED_CHUNK_SIZE: Vec3 = CHUNK_SIZE * crate::voxel::GRID_SCALE;

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
            // const CHUNK_SIZE: Vec3 = Vec3::splat(unpadded::SIZE as f32);

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
            let voxel = self.get_voxel(hit.voxel);
            if voxel.pickable() {
                return Some(hit);
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
                Voxel::Air => {
                    while heights[index] > *height_range.start() {
                        match self.get_voxel(voxel + IVec3::Y * heights[index]) {
                            Voxel::Air => {},
                            _ => break,
                        }

                        heights[index] -= 1;
                    }
                },
                _ => {
                    while heights[index] < *height_range.end() {
                        match self.get_voxel(voxel + IVec3::Y * heights[index]) {
                            Voxel::Air => break,
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
            let voxel = self.get_voxel(pos + IVec3::Y * height);
            gradient.push(voxel);
        }

        if new_height < pos.y {
            // squish the voxels down
        } else {
            // stretch the voxels out
        }
    }

    pub fn diff(&self, other: &Voxels, cutoff: usize) -> Vec<VoxelDiff> {
        let mut diffs = Vec::new();

        for point in self.point_iter() {
            let v1 = self.get_voxel(point);
            let v2 = other.get_voxel(point);
            if v1 != v2 {
                diffs.push(VoxelDiff { point, v1, v2 });
                if diffs.len() > cutoff {
                    break;
                }
            }
        }

        return diffs;
    }
}

#[derive(Debug, Copy, Clone)]
pub struct VoxelDiff {
    pub point: IVec3,
    pub v1: Voxel,
    pub v2: Voxel,
}

#[cfg(test)]
pub mod tests {
    use super::*;

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

    #[test]
    fn set_voxel_batch() {
        // just make sure the batch actually does the same thing as setting directly
        let size = 1;
        let len = size * size * size;
        let point_iter = (-size..=size).flat_map(move |y| {
            (-size..=size).flat_map(move |x| (-size..=size).map(move |z| IVec3::new(x, y, z)))
        });
        let voxel_iter = (-len..len).map(|_| Voxel::Sand);

        let mut voxels_direct = Voxels::new();
        for (point, voxel) in point_iter.clone().zip(voxel_iter.clone()) {
            voxels_direct.set_voxel(point, voxel);
        }

        let mut voxels_batch = Voxels::new();
        voxels_batch.set_voxels(point_iter.clone(), voxel_iter.clone());

        let diff = voxels_direct.diff(&voxels_batch, 50);
        if diff.len() > 0 {
            panic!("diffs: {:?}", diff);
        }
    }
}
