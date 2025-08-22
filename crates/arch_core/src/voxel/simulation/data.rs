use bevy::platform::collections::hash_map::IterMut;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
#[cfg(feature = "trace")]
use tracing::*;

use crate::voxel::simulation::SimSwapBuffer;
use crate::voxel::voxel::VoxelChangeset;
use crate::voxel::{Voxel, Voxels};

pub const CHUNK_WIDTH_BITSHIFT: usize = 4;
pub const CHUNK_WIDTH_BITSHIFT_Y: usize = CHUNK_WIDTH_BITSHIFT * 2;
pub const CHUNK_REMAINDER: i32 = (CHUNK_WIDTH - 1) as i32;
pub const CHUNK_WIDTH: usize = 1 << CHUNK_WIDTH_BITSHIFT;
pub const CHUNK_LENGTH: usize = CHUNK_WIDTH * CHUNK_WIDTH * CHUNK_WIDTH;

pub type ChunkPoint = IVec3;

pub fn plugin(app: &mut App) {
    app.register_type::<SimChunk>();

    app.add_observer(insert_voxels_sim_chunks);
}

pub fn insert_voxels_sim_chunks(
    trigger: Trigger<OnInsert, Voxels>,
    mut commands: Commands,
    voxels: Query<&Voxels>,
) {
    let Ok(voxels) = voxels.get(trigger.target()) else {
        return;
    };
    // println!("adding sim chunks");
    let sim_swap_buffer = SimSwapBuffer(voxels.sim_chunks.create_update_buffer());
    commands.entity(trigger.target()).insert(sim_swap_buffer);
}

// #[derive(Debug, Clone, PartialEq, Eq, Reflect)]
pub type UpdateBuffer = HashMap<ChunkPoint, [u64; CHUNK_LENGTH / 64]>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub struct SimChunk {
    pub voxel_changeset: VoxelChangeset,
    // lets try just a 4x4x4 chunk
    pub voxels: [u16; CHUNK_LENGTH],
}

impl Default for SimChunk {
    fn default() -> Self {
        Self { voxel_changeset: default(), voxels: [0; CHUNK_LENGTH] }
    }
}

impl SimChunk {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
pub struct SimChunks {
    // pub chunks: Vec<SimChunk>,
    // pub chunk_strides: [usize; 3],
    pub chunks: HashMap<ChunkPoint, SimChunk>,

    pub sim_updates: UpdateBuffer, // bitmask of updates
    // pub render_updates: UpdateBuffer, // bitmask of updates
    // pub updated_chunks: HashSet<ChunkPoint>,
    pub chunk_size: IVec3,
    pub voxel_size: IVec3,
}

#[inline]
pub fn to_linear_index(relative_point: IVec3) -> usize {
    debug_assert!(
        relative_point.x < CHUNK_WIDTH as i32
            && relative_point.y < CHUNK_WIDTH as i32
            && relative_point.z < CHUNK_WIDTH as i32
            && relative_point.x >= 0
            && relative_point.y >= 0
            && relative_point.z >= 0 /* ((relative_point.x | relative_point.y |
                                      * relative_point.z) & !15) != 0 */
    );

    // z + x * 4 + y * 16
    // zxy order for now, maybe check if yxz is better later since the most checks
    // are vertical
    (relative_point.z
        + (relative_point.x << CHUNK_WIDTH_BITSHIFT)
        + (relative_point.y << CHUNK_WIDTH_BITSHIFT_Y)) as usize
}

#[inline]
pub fn from_linear_index(index: usize) -> IVec3 {
    debug_assert!(index < CHUNK_LENGTH);

    let mut index = index as i32;
    let y = index >> CHUNK_WIDTH_BITSHIFT_Y;
    index -= y << CHUNK_WIDTH_BITSHIFT_Y;
    let x = index >> CHUNK_WIDTH_BITSHIFT;
    let z = index & CHUNK_REMAINDER; // index % CHUNK_WIDTH 
    ivec3(x, y, z)
}

#[inline]
pub fn linearize(relative_point: IVec3) -> usize {
    to_linear_index(relative_point)
    // super::morton::to_morton_index_lut(relative_point)
    // unsafe { super::morton::into_morton_index_bmi2(relative_point) }
}

#[inline]
pub fn delinearize(index: usize) -> IVec3 {
    from_linear_index(index)
    // super::morton::from_morton_index(index)
    // unsafe { super::morton::from_morton_index_bmi2(index) }
}

#[inline]
pub fn chunk_point(point: IVec3) -> ChunkPoint {
    // not euclidean (point / 16)
    point >> (CHUNK_WIDTH_BITSHIFT as i32)
}

pub struct UpdateIterator<'a> {
    // pub chunk_updates: &'a mut UpdateBuffer,
    // pub chunk_points: Vec<IVec3>,
    // pub chunk_index: usize,
    pub iter: IterMut<'a, IVec3, [u64; CHUNK_LENGTH / 64]>,
    pub current_mask: Option<(&'a IVec3, &'a mut [u64; CHUNK_LENGTH / 64])>,
    pub mask_index: usize,
}

impl<'a> Iterator for UpdateIterator<'a> {
    type Item = (IVec3, usize);

    // (chunk_index, voxel_index)

    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(feature = "trace")]
        let span = info_span!("UpdateIterator.next").entered();

        const MASK_LENGTH: usize = CHUNK_LENGTH / 64;
        // println!("UPDATE ITERATOR NEXT");
        while self.mask_index < MASK_LENGTH && self.current_mask.is_some() {
            // let chunk_point = self.chunk_points[self.chunk_index];
            // let mut chunk_updates = self.chunk_updates.get_mut(&chunk_point).unwrap();
            let (chunk_point, chunk_bitsets) = self.current_mask.as_mut().unwrap();

            let bitset = if cfg!(feature = "safe-bounds") {
                &mut chunk_bitsets[self.mask_index]
            } else {
                unsafe { chunk_bitsets.get_unchecked_mut(self.mask_index) }
            };

            if *bitset != 0 {
                // `bitset & -bitset` returns a bitset with only the lowest significant bit set
                let t = *bitset & bitset.wrapping_neg();
                let trailing = bitset.trailing_zeros() as usize;
                let voxel_index = self.mask_index * 64 + trailing;
                *bitset ^= t;
                return Some((**chunk_point, voxel_index));
            } else {
                self.mask_index += 1;
                if self.mask_index == MASK_LENGTH {
                    #[cfg(feature = "trace")]
                    let span = info_span!("UpdateIterator.next_internal").entered();
                    self.mask_index = 0;
                    self.current_mask = self.iter.next();
                }
            }
        }

        None
    }
}

impl SimChunks {
    pub fn new(voxel_size: IVec3) -> Self {
        let chunk_size = voxel_size / CHUNK_WIDTH as i32;
        Self {
            // chunks: vec![SimChunk::new(); (chunk_size.x * chunk_size.y * chunk_size.z) as usize],
            chunks: HashMap::with_capacity((chunk_size.x * chunk_size.y * chunk_size.z) as usize),
            sim_updates: Self::create_update_buffer_from_size(chunk_size),
            // updated_chunks: HashSet::new(),
            // chunk_strides,
            chunk_size,
            voxel_size,
        }
    }

    pub fn create_update_buffer_from_size(chunk_size: IVec3) -> UpdateBuffer {
        info!(
            "creating update buffer from size: {:?} -> {}",
            chunk_size,
            (chunk_size.x * chunk_size.y * chunk_size.z) as usize
        );
        // vec![[0; CHUNK_LENGTH / 64]; (chunk_size.x * chunk_size.y * chunk_size.z) as
        // usize]
        HashMap::with_capacity((chunk_size.x * chunk_size.y * chunk_size.z) as usize)
    }

    pub fn create_update_buffer(&self) -> UpdateBuffer {
        Self::create_update_buffer_from_size(self.chunk_size)
    }

    // #[inline]
    // pub fn chunk_linearize(&self, chunk_point: IVec3) -> usize {
    //     chunk_point.x as usize
    //         + chunk_point.z as usize * self.chunk_strides[1]
    //         + chunk_point.y as usize * self.chunk_strides[2]
    // }

    // #[inline]
    // pub fn chunk_delinearize(&self, mut chunk_index: usize) -> IVec3 {
    //     let y = chunk_index / self.chunk_strides[2];
    //     chunk_index -= y * self.chunk_strides[2];
    //     let z = chunk_index / self.chunk_strides[1];
    //     let x = chunk_index % self.chunk_strides[1];
    //     ivec3(x as i32, y as i32, z as i32)
    // }

    #[inline]
    pub fn in_bounds(&self, point: IVec3) -> bool {
        point.x >= 0
            && point.y >= 0
            && point.z >= 0
            && point.x < self.voxel_size.x
            && point.y < self.voxel_size.y
            && point.z < self.voxel_size.z
        // ((relative_point.x | relative_point.y | relative_point.z) & !15) != 0
    }

    #[inline]
    pub fn get_voxel_from_indices(&self, chunk_point: ChunkPoint, voxel_index: usize) -> Voxel {
        #[cfg(feature = "trace")]
        let span = info_span!("get_voxel_from_indices").entered();

        let Some(chunk) = self.chunks.get(&chunk_point) else {
            return Voxel::Air;
        };

        let voxel = if cfg!(feature = "safe-bounds") {
            chunk.voxels[voxel_index]
        } else {
            unsafe { *chunk.voxels.get_unchecked(voxel_index) }
        };

        Voxel::from_data(voxel)
    }

    #[inline]
    pub fn get_voxel(&self, point: IVec3) -> Voxel {
        if !self.in_bounds(point) {
            return Voxel::Barrier;
        }

        let (chunk_index, voxel_index) = Self::chunk_and_voxel_indices(point);
        self.get_voxel_from_indices(chunk_index, voxel_index)
    }

    #[inline]
    pub fn set_voxel(&mut self, point: IVec3, voxel: Voxel) {
        if !self.in_bounds(point) {
            return;
        }

        let (chunk_point, voxel_index) = Self::chunk_and_voxel_indices(point);
        let chunk = self.chunks.entry(chunk_point).or_default();
        if cfg!(feature = "safe-bounds") {
            chunk.voxels[voxel_index] = voxel.data();
        } else {
            unsafe {
                *chunk.voxels.get_unchecked_mut(voxel_index) = voxel.data();
            }
        }

        chunk.voxel_changeset.set(voxel);

        // self.updated_chunks.insert(chunk_point);
        self.push_neighbor_sim_updates(point);
    }

    pub fn set_voxel_aabb(&mut self, aabb: crate::voxel::voxel_aabb::VoxelAabb, voxel: Voxel) {
        // Iterate over all chunk coordinates that intersect the AABB
        let min = aabb.min;
        let max = aabb.max;

        // Compute chunk bounds
        let chunk_min = (min - IVec3::ONE).div_euclid(IVec3::splat(CHUNK_WIDTH as i32));
        let chunk_max = (max + IVec3::ONE).div_euclid(IVec3::splat(CHUNK_WIDTH as i32));

        for cz in chunk_min.z..=chunk_max.z {
            for cx in chunk_min.x..=chunk_max.x {
                for cy in chunk_min.y..=chunk_max.y {
                    let chunk_point = IVec3::new(cx, cy, cz);

                    // Compute the voxel-space bounds for this chunk
                    let chunk_voxel_min = chunk_point * CHUNK_WIDTH as i32;
                    let chunk_voxel_max =
                        chunk_voxel_min + IVec3::splat(CHUNK_WIDTH as i32) - IVec3::ONE;

                    // Clamp the affected region to the intersection of the chunk and the AABB
                    let set_min = chunk_voxel_min.max(min);
                    let set_max = chunk_voxel_max.min(max);
                    let update_min = chunk_voxel_min.max(min - IVec3::ONE);
                    let update_max = chunk_voxel_max.min(max + IVec3::ONE);

                    // Get or create the chunk
                    let chunk = self.chunks.entry(chunk_point).or_default();

                    chunk.voxel_changeset.set(voxel);

                    for z in set_min.z..=set_max.z {
                        for x in set_min.x..=set_max.x {
                            for y in set_min.y..=set_max.y {
                                let local = IVec3::new(x, y, z) - chunk_voxel_min;
                                let voxel_index = to_linear_index(local);

                                if cfg!(feature = "safe-bounds") {
                                    chunk.voxels[voxel_index] = voxel.data();
                                } else {
                                    unsafe {
                                        *chunk.voxels.get_unchecked_mut(voxel_index) = voxel.data();
                                    }
                                }
                            }
                        }
                    }

                    for z in update_min.z..=update_max.z {
                        for x in update_min.x..=update_max.x {
                            for y in update_min.y..=update_max.y {
                                let local = IVec3::new(x, y, z) - chunk_voxel_min;
                                let voxel_index = to_linear_index(local);

                                // Mark for simulation update
                                Self::add_update_mask(
                                    &mut self.sim_updates,
                                    chunk_point,
                                    voxel_index,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[inline]
    pub fn push_neighbor_sim_updates(&mut self, point: IVec3) {
        for y in -1..=1 {
            for x in -1..=1 {
                for z in -1..=1 {
                    let offset = ivec3(x, y, z);
                    let neighbor = point + offset;
                    // if self.get_voxel(neighbor).is_simulated() {
                    //     self.push_sim_update(neighbor);
                    // }
                    // self.updated_chunks.insert(chunk_point(neighbor));
                    self.push_sim_update(neighbor);
                }
            }
        }
    }

    #[inline]
    pub fn push_sim_update(&mut self, point: IVec3) {
        if self.in_bounds(point) {
            let (chunk_index, voxel_index) = Self::chunk_and_voxel_indices(point);
            Self::add_update_mask(&mut self.sim_updates, chunk_index, voxel_index);
        }
    }

    #[inline]
    pub fn add_update_mask(mask: &mut UpdateBuffer, chunk_point: ChunkPoint, voxel_index: usize) {
        // info!("adding update mask: {:?}", (chunk_index, voxel_index));
        let mask_index = voxel_index >> 6; // voxel_index / 64
        let bit_index = voxel_index & 63; // voxel_index % 64

        let chunk_mask = mask.entry(chunk_point).or_insert([0u64; CHUNK_LENGTH / 64]);
        if cfg!(feature = "safe-bounds") {
            chunk_mask[mask_index] |= 1 << bit_index;
        } else {
            unsafe {
                // 5% faster, use when we feel fine with callers of this
                *chunk_mask.get_unchecked_mut(mask_index) |= 1 << bit_index;
            }
        }
    }

    pub fn clear_updates(&mut self) {
        self.sim_updates.clear();
    }

    pub fn sim_updates<'a, 'b: 'a>(
        &mut self,
        swap_buffer: &'b mut UpdateBuffer,
    ) -> UpdateIterator<'a> {
        #[cfg(feature = "trace")]
        let span = info_span!("sim_updates").entered();
        // debug_assert_eq!(self.sim_updates.len(), swap_buffer.len());
        std::mem::swap(&mut self.sim_updates, swap_buffer);
        let mut iter = swap_buffer.iter_mut();
        let first = iter.next();
        UpdateIterator { iter, current_mask: first, mask_index: 0 }
    }

    #[inline]
    pub fn chunk_and_voxel_indices(point: IVec3) -> (ChunkPoint, usize) {
        // chunk index
        let chunk_point = chunk_point(point);

        // voxel index
        let relative_voxel_point = point - (chunk_point << (CHUNK_WIDTH_BITSHIFT as i32));
        let voxel_index = linearize(relative_voxel_point);

        (chunk_point, voxel_index)
    }

    pub fn point_from_chunk_and_voxel_indices(
        chunk_point: ChunkPoint,
        voxel_index: usize,
    ) -> IVec3 {
        #[cfg(feature = "trace")]
        let span = info_span!("point_from_chunk_and_voxel_indices").entered();

        // let chunk_point = self.chunk_delinearize(chunk_index);
        let relative_voxel_point = delinearize(voxel_index);
        (chunk_point << (CHUNK_WIDTH_BITSHIFT as i32)) + relative_voxel_point
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn chunk_linearize() {
    //     let chunks = SimChunks::new(ivec3(64, 64, 64));
    //     assert_eq!(chunks.chunk_delinearize(0), ivec3(0, 0, 0));
    //     assert_eq!(
    //         chunks.chunk_delinearize(chunks.chunk_linearize(ivec3(0, 0, 0))),
    //         ivec3(0, 0, 0)
    //     );
    //     assert_eq!(
    //         chunks.chunk_delinearize(chunks.chunk_linearize(ivec3(1, 1, 1))),
    //         ivec3(1, 1, 1)
    //     );
    // }

    #[test]
    fn chunk_voxel_indices() {
        let chunks = SimChunks::new(ivec3(64, 64, 64));
        // chunks.chunk_and_voxel_indices(ivec3(12, 12, 12));
        // assert_eq!(chunks.chunk_and_voxel_indices(ivec3(0, 0, 0)), (0, 0));
        // assert_eq!(chunks.chunk_and_voxel_indices(ivec3(16, 0, 0)), (1, 0));
        // assert_eq!(chunks.chunk_and_voxel_indices(ivec3(16, 0, 5)), (1, 0));

        let sanity = |p: IVec3| -> IVec3 {
            let (chunk_index, voxel_index) = SimChunks::chunk_and_voxel_indices(p);
            SimChunks::point_from_chunk_and_voxel_indices(chunk_index, voxel_index)
        };

        assert_eq!(sanity(ivec3(0, 0, 0)), ivec3(0, 0, 0));
        assert_eq!(sanity(ivec3(12, 12, 12)), ivec3(12, 12, 12));
        assert_eq!(sanity(ivec3(63, 63, 63)), ivec3(63, 63, 63));
        assert_eq!(sanity(ivec3(1, 0, 0)), ivec3(1, 0, 0));
        assert_eq!(sanity(ivec3(0, 1, 0)), ivec3(0, 1, 0));
        assert_eq!(sanity(ivec3(0, 0, 1)), ivec3(0, 0, 1));
        assert_eq!(sanity(ivec3(63, 0, 0)), ivec3(63, 0, 0));
        assert_eq!(sanity(ivec3(0, 63, 0)), ivec3(0, 63, 0));
        assert_eq!(sanity(ivec3(0, 0, 63)), ivec3(0, 0, 63));
    }

    // #[test]
    // fn point_linearize() {
    //     let mut chunks = SimChunks::new(ivec3(16, 1, 16));
    //     chunks.linearize(ivec3(0,0,0));

    //     assert_eq!(bucket_linearize(ivec3(0, 0, 0)), 0);
    //     assert_eq!(bucket_linearize(ivec3(3, 0, 0)), 3);
    //     assert_eq!(bucket_linearize(ivec3(0, 0, 3)), 12);
    //     assert_eq!(bucket_linearize(ivec3(0, 3, 0)), 48);
    //     assert_eq!(bucket_linearize(ivec3(3, 3, 3)), 63);
    // }

    #[test]
    fn get_set() {
        let mut chunks = SimChunks::new(ivec3(16, 1, 16));

        // basic set
        chunks.set_voxel(ivec3(0, 0, 0), Voxel::Dirt);
        assert_eq!(chunks.get_voxel(ivec3(0, 0, 0)), Voxel::Dirt);

        // oob
        chunks.set_voxel(ivec3(-1, 0, 0), Voxel::Dirt);
        assert_eq!(chunks.get_voxel(ivec3(-1, 0, 0)), Voxel::Barrier);

        // voxel data
        chunks.set_voxel(ivec3(1, 0, 0), Voxel::Water(default()));
        assert_eq!(chunks.get_voxel(ivec3(1, 0, 0)), Voxel::Water(default()));
    }

    // #[test]
    // fn update_iterator() {
    //     let mut chunks = SimChunks::new(ivec3(32, 32, 32));
    //     chunks.push_neighbor_sim_updates(ivec3(0, 0, 0));
    //     let mut buffer = chunks.create_update_buffer();
    //     let updates = chunks.sim_updates(&mut buffer);
    //     for (chunk_index, voxel_index) in updates {
    //         println!("chunk_index: {}, voxel_index: {}", chunk_index,
    // voxel_index);     }

    //     SimChunks::add_update_mask(&mut chunks.sim_updates, IVec3::ZERO, 0);
    //     SimChunks::add_update_mask(&mut chunks.sim_updates, IVec3::ZERO, 100);
    //     let updates = chunks.sim_updates(&mut buffer);
    //     println!("second round");
    //     for (chunk_index, voxel_index) in updates {
    //         println!("chunk_index: {}, voxel_index: {}", chunk_index,
    // voxel_index);     }

    //     println!("renderin round");
    //     for (chunk_index, voxel_index) in chunks.render_updates(&mut buffer) {
    //         println!("chunk_index: {}, voxel_index: {}", chunk_index,
    // voxel_index);     }
    //     // assert_eq!(updates.next(), Some((0, 0)));
    //     // assert_eq!(updates.next(), None);
    // }

    #[test]
    fn test_shift() {
        // reference so i stop fucking up directions of bitshifts lmao
        let x = 2;
        println!("{}", x << 1); // multiply by 2
        println!("{}", x >> 1); // divide by 2
    }
}
