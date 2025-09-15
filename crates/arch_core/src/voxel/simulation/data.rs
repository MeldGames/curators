use std::hash::{Hash, Hasher};

use bevy::platform::collections::hash_map::IterMut;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;
use fxhash::FxHashMap;
use slotmap::SlotMap;
#[cfg(feature = "trace")]
use tracing::*;

use crate::sdf::Sdf;
use crate::voxel::simulation::FallingSandTick;
use crate::voxel::simulation::rle::RLEChunk;
use crate::voxel::voxel::VoxelSet;
use crate::voxel::{Voxel, Voxels};

pub const CHUNK_WIDTH_BITSHIFT: usize = 4;
pub const CHUNK_WIDTH_BITSHIFT_Y: usize = CHUNK_WIDTH_BITSHIFT * 2;
pub const CHUNK_REMAINDER: usize = CHUNK_WIDTH - 1;
pub const CHUNK_WIDTH: usize = 1 << CHUNK_WIDTH_BITSHIFT;
pub const CHUNK_LENGTH: usize = CHUNK_WIDTH * CHUNK_WIDTH * CHUNK_WIDTH;

pub const CHUNK_VIEW_SIZE: usize = 2;
pub const CHUNK_VIEW_LENGTH: usize = CHUNK_VIEW_SIZE * CHUNK_VIEW_SIZE * CHUNK_VIEW_SIZE;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Reflect, Deref, DerefMut, Hash)]
pub struct ChunkPoint(pub IVec3);

pub fn plugin(app: &mut App) {
    app.register_type::<SimChunk>();
}

pub struct DirtyReader<'a> {
    set: &'a DirtySet,
    mask_index: u64,
    current_mask: u64,
}

impl<'a> Iterator for DirtyReader<'a> {
    type Item = u64; // voxel index
    fn next(&mut self) -> Option<Self::Item> {
        while self.mask_index < CHUNK_LENGTH as u64 / 64 {
            if self.current_mask != 0 {
                // `bitset & -bitset` returns a bitset with only the lowest significant bit set
                let t = self.current_mask & self.current_mask.wrapping_neg();
                let trailing = self.current_mask.trailing_zeros() as u64;
                let voxel_index = self.mask_index * 64 + trailing;
                self.current_mask ^= t;
                return Some(voxel_index);
            } else {
                self.mask_index += 1;

                self.current_mask = if cfg!(feature = "safe-bounds") {
                    self.set.set[self.mask_index as usize]
                } else {
                    unsafe { *self.set.set.get_unchecked(self.mask_index as usize) }
                };
            }
        }

        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub struct DirtySet {
    set: [u64; CHUNK_LENGTH / 64],
}

impl DirtySet {
    pub fn filled() -> Self {
        Self {
            set: [u64::MAX; CHUNK_LENGTH / 64],
        }
    }

    pub fn empty() -> Self {
        Self {
            set: [0u64; CHUNK_LENGTH / 64],
        }
    }

    pub fn clear(&mut self) {
        for mask in &mut self.set {
            *mask = 0;
        }
    }

    pub fn set(&mut self, voxel_index: usize) {
        let mask = voxel_index / 64;
        let bit = voxel_index % 64;
        self.set[mask] |= 1 << bit;
    }

    // set neighbors of a voxel that is fully self-contained in this chunk
    pub fn set_neighbors(&mut self, voxel_index: usize) {
        for z in -1..1 {
            for x in -1..1 {
                for y in -1..1 {
                    self.set(voxel_index + linearize(IVec3::new(x, y, z)));
                }
            }
        }
    }

    // TODO: figure out how to spread in x, y, z directions to neighbors
    // pub fn spread(&mut self) {
    //     for mask_index in 0..CHUNK_LENGTH / 64{
    //         let z_adjacent = self.set[mask_index - 1];
    //         let z_adjacent_2 = self.set[mask_index + 1];
    //         // spread z
    //         self.set[mask_index] |= (*mask << 1) | (*mask >> 1);
    //         // spread x
    //         // spread on lower layer
    //         *mask |= (*mask << 16) | (*mask >> 16);
    //         // spread on surrounding
    //     }
    // }

    pub fn display(&self) -> String {
        let mut layers = String::new();
        for mask in self.set {
            layers += &format!("\n{:0b}", mask);
        }
        layers
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub struct SimChunk {
    // pub voxel_changeset: VoxelSet,
    /// Voxels that are considered dirty still.
    pub dirty: DirtySet,
    // lets try just a 4x4x4 chunk
    pub voxels: [Voxel; CHUNK_LENGTH],
}

impl Default for SimChunk {
    fn default() -> Self {
        Self::fill(Voxel::Air)
    }
}

impl SimChunk {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn fill(voxel: Voxel) -> Self {
        Self {
            dirty: DirtySet::empty(),
            voxels: [voxel; CHUNK_LENGTH],
        }
    }
}

slotmap::new_key_type! { pub struct ChunkKey; }

// TODO: Double buffer this data so we can throw this on another thread,
// and we can read the current chunks from the last sim
#[derive(Component, Debug, Clone)]
pub struct SimChunks {
    pub chunks: SlotMap<ChunkKey, SimChunk>, // active chunks

    pub from_chunk_point: HashMap<ChunkPoint, ChunkKey>,

    /// 0..8 offsets
    pub margolus_offset: usize,
}

// TODO: Figure out better looking offsets.
// Probably keep alternating the y positions each offset.
pub const MARGOLUS_OFFSETS: [IVec3; 8] = [
    IVec3::new(0, 0, 0),
    IVec3::new(1, 1, 0),
    IVec3::new(0, 0, 1),
    IVec3::new(0, 1, 1),
    IVec3::new(1, 0, 0),
    IVec3::new(0, 1, 0),
    IVec3::new(1, 0, 1),
    IVec3::new(1, 1, 1),
];

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

    let y = index >> CHUNK_WIDTH_BITSHIFT_Y;
    let x = (index >> CHUNK_WIDTH_BITSHIFT) & CHUNK_REMAINDER;
    let z = index & CHUNK_REMAINDER; // index % CHUNK_WIDTH 

    ivec3(x as i32, y as i32, z as i32)
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
pub fn voxel_relative_point(point: IVec3) -> IVec3 {
    point.rem_euclid(IVec3::splat(CHUNK_WIDTH as i32))
}

#[inline]
pub fn chunk_point(point: IVec3) -> ChunkPoint {
    // not euclidean (point / 16)
    // ChunkPoint(point >> (CHUNK_WIDTH_BITSHIFT as i32))

    ChunkPoint(point.div_euclid(IVec3::splat(CHUNK_WIDTH as i32)))
}

// impl<'a> Iterator for UpdateIterator<'a> {
// type Item = (ChunkPoint, usize);
//
// (chunk_index, voxel_index)
//
// fn next(&mut self) -> Option<Self::Item> {
// #[cfg(feature = "trace")]
// let span = info_span!("UpdateIterator.next").entered();
//
// const MASK_LENGTH: usize = CHUNK_LENGTH / 64;
// println!("UPDATE ITERATOR NEXT");
// while self.mask_index < MASK_LENGTH && self.current_mask.is_some() {
// let chunk_point = self.chunk_points[self.chunk_index];
// let mut chunk_updates = self.chunk_updates.get_mut(&chunk_point).unwrap();
// let (chunk_point, chunk_bitsets) = self.current_mask.as_mut().unwrap();
//
// let bitset = if cfg!(feature = "safe-bounds") {
// &mut chunk_bitsets[self.mask_index]
// } else {
// unsafe { chunk_bitsets.get_unchecked_mut(self.mask_index) }
// };
//
// if *bitset != 0 {
// `bitset & -bitset` returns a bitset with only the lowest significant bit set
// let t = *bitset & bitset.wrapping_neg();
// let trailing = bitset.trailing_zeros() as usize;
// let voxel_index = self.mask_index * 64 + trailing;
// bitset ^= t;
// return Some((**chunk_point, voxel_index));
// } else {
// self.mask_index += 1;
// if self.mask_index == MASK_LENGTH {
// #[cfg(feature = "trace")]
// let span = info_span!("UpdateIterator.next_internal").entered();
// self.mask_index = 0;
// self.current_mask = self.iter.next();
// }
// }
// }
//
// None
// }
// }

impl SimChunks {
    pub fn new() -> Self {
        Self { chunks: SlotMap::with_key(), from_chunk_point: HashMap::new(), margolus_offset: 0 }
    }

    #[inline]
    pub fn chunk_and_voxel_indices(point: IVec3) -> (ChunkPoint, usize) {
        // chunk index
        let chunk_point = chunk_point(point);

        // voxel index
        let relative_voxel_point = point - (chunk_point.0 << (CHUNK_WIDTH_BITSHIFT as i32));
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
        (chunk_point.0 << (CHUNK_WIDTH_BITSHIFT as i32)) + relative_voxel_point
    }

    pub fn add_chunk(&mut self, chunk_point: ChunkPoint, sim_chunk: SimChunk) {
        let mut existing_chunk = None;

        if let Some(chunk_key) = self.from_chunk_point.get(&chunk_point) {
            if let Some(chunk) = self.chunks.get_mut(*chunk_key) {
                existing_chunk = Some(chunk);
            }
        }

        if let Some(existing_chunk) = existing_chunk {
            *existing_chunk = sim_chunk;
        } else {
            // drop(existing_chunk);
            let chunk_key = self.chunks.insert(sim_chunk);
            self.from_chunk_point.insert(chunk_point, chunk_key);
        }
    }

    #[inline]
    pub fn chunk_key_from_point(&self, chunk_point: ChunkPoint) -> Option<ChunkKey> {
        self.from_chunk_point.get(&chunk_point).copied()
    }

    #[inline]
    pub fn get_voxel_from_indices(&self, chunk_key: ChunkKey, voxel_index: usize) -> Voxel {
        #[cfg(feature = "trace")]
        let span = info_span!("get_voxel_from_indices").entered();

        let Some(chunk) = self.chunks.get(chunk_key) else {
            return Voxel::Air;
        };

        let voxel = if cfg!(feature = "safe-bounds") {
            chunk.voxels[voxel_index]
        } else {
            unsafe { *chunk.voxels.get_unchecked(voxel_index) }
        };

        voxel
    }

    #[inline]
    pub fn get_voxel(&self, point: IVec3) -> Option<Voxel> {
        let (chunk_point, voxel_index) = Self::chunk_and_voxel_indices(point);
        if let Some(chunk_key) = self.chunk_key_from_point(chunk_point) {
            Some(self.get_voxel_from_indices(chunk_key, voxel_index))
        } else {
            None
        }
    }

    #[inline]
    pub fn set_voxel(&mut self, point: IVec3, voxel: Voxel) -> bool {
        let (chunk_point, voxel_index) = Self::chunk_and_voxel_indices(point);
        if let Some(chunk_key) = self.chunk_key_from_point(chunk_point) {
            let chunk = self.chunks.get_mut(chunk_key).unwrap();

            if cfg!(feature = "safe-bounds") {
                chunk.voxels[voxel_index] = voxel;
            } else {
                unsafe {
                    *chunk.voxels.get_unchecked_mut(voxel_index) = voxel;
                }
            }

            // chunk.voxel_changeset.set(voxel);

            true
        } else {
            false
        }

        // self.updated_chunks.insert(chunk_point);
        // self.push_neighbor_sim_updates(point);
    }

    pub fn set_voxel_brush<S: Sdf>(&mut self, center: IVec3, brush: S, voxel: Voxel) {
        let half_size = Vec3A::splat(500.0);
        for raster_voxel in crate::sdf::voxel_rasterize::rasterize(
            brush,
            crate::sdf::voxel_rasterize::RasterConfig {
                clip_bounds: Aabb3d { min: -half_size, max: half_size },
                grid_scale: crate::voxel::GRID_SCALE,
                pad_bounds: Vec3::ZERO,
            },
        ) {
            if raster_voxel.distance <= 0.0 {
                self.set_voxel(raster_voxel.point + center, voxel);
            }
        }
    }

    /// Create a 2x2x2 area of chunks based on the current margolus offset.
    pub fn construct_blocks(&self) -> Vec<ChunkKeys> {
        use bevy::platform::collections::hash_map::Entry;

        let mut to_index: HashMap<ChunkPoint, usize> = HashMap::new();
        let mut blocks: Vec<ChunkKeys> = Vec::new();

        let offset = MARGOLUS_OFFSETS[self.margolus_offset];
        for (&chunk_point, &chunk_key) in self.from_chunk_point.iter() {
            let anchor = (*chunk_point + offset) / 2;

            // Linearize the chunk position into a 2x2x2 block (x, y, z in {0,1})
            let rel = *chunk_point - anchor * 2;
            let chunk_index = ChunkView::linearize_chunk(rel);

            let block_index = match to_index.entry(ChunkPoint(anchor)) {
                Entry::Occupied(entry) => *entry.get(),
                Entry::Vacant(entry) => {
                    blocks.push(ChunkKeys { start_chunk_point: anchor, keys: [None; 8] });

                    let block_index = blocks.len() - 1;
                    entry.insert(block_index);
                    block_index
                },
            };

            let block = blocks.get_mut(block_index).unwrap();
            block.keys[chunk_index] = Some(chunk_key);
        }

        blocks
    }

    /// Split the [`SimChunks`] into a list of [`ChunkView`]s that can be
    /// processed in parallel.
    pub fn chunk_views<'a>(&'a mut self) -> Vec<ChunkView<'a>> {
        let blocks = self.construct_blocks();
        let mut views = blocks.iter().map(|keys| ChunkView {
            start_chunk_point: keys.start_chunk_point,
            chunks: std::array::from_fn(|_| None),
        }).collect::<Vec<_>>();

        // block_index, key_index, key
        let flattened = blocks.iter()
            .enumerate()
            .flat_map(|(block_index, block)| {
                block.keys.iter()
                    .enumerate()
                    .filter_map(move |(key_index, key)| {
                        key.map(|key| (block_index, key_index, key))
                    })
            })
            .collect::<Vec<_>>();
        
        let keys = flattened.iter().map(|(_, _, key)| *key).collect::<Vec<_>>();
        let disjoint = unsafe { self.get_disjoint_mut_unchecked(&keys.as_slice()) };
        for (chunk_ref, (block_index, key_index, key)) in disjoint.into_iter().zip(flattened) {
            views[block_index].chunks[key_index] = Some(chunk_ref);
        }

        views
        /*
        unsafe {
            if let Some(disjoint_chunks) = self.get_disjoint_blocks_mut(blocks.as_slice()) {
                disjoint_chunks
                    .into_iter()
                    .zip(blocks)
                    .map(|(block, keys)| ChunkView {
                        start_chunk_point: keys.start_chunk_point,
                        chunks: block,
                    })
                    .collect::<Vec<_>>()
            } else {
                panic!("Some blocks were joint");
            }
        }
        */
    }

    // TODO: Fix this, it currently leads to a double free issue.

    /// # Safety
    /// - The caller must guarantee that all `keys` are valid and unique (disjoint).
    pub unsafe fn get_disjoint_mut_unchecked<'a>(&'a mut self, keys: &[ChunkKey]) -> Vec<&'a mut SimChunk> {
        let mut result = Vec::with_capacity(keys.len());
        for &key in keys {
            // SAFETY: The caller must guarantee that all keys are valid and disjoint.
            let ptr = self.chunks.get_unchecked_mut(key) as *mut SimChunk;
            result.push(&mut *ptr);
        }
        result
    }
    /*
    pub unsafe fn get_disjoint_blocks_mut(
        &mut self,
        blocks: &[ChunkKeys],
    ) -> Option<Vec<[Option<&mut SimChunk>; CHUNK_VIEW_LENGTH]>> {
        let mut ptrs: Vec<[Option<*mut SimChunk>; CHUNK_VIEW_LENGTH]> =
            vec![[None; CHUNK_VIEW_LENGTH]; blocks.len()];

        // Verify chunk keys are already aliased
        let mut aliased = HashSet::new();

        // Safety: Each chunk key should only be aliased once, otherwise we return early
        // and no references can be used.
        unsafe {
            for (block_index, block) in blocks.iter().enumerate() {
                for (chunk_index, chunk_key) in block.keys.iter().enumerate() {
                    if let Some(chunk_key) = chunk_key {
                        ptrs[block_index][chunk_index] =
                            self.chunks.get_mut(*chunk_key).map(|s| s as *mut SimChunk);

                        if aliased.contains(&chunk_key) {
                            return None;
                        } else {
                            aliased.insert(chunk_key);
                        }
                    }
                }
            }

            Some(core::mem::transmute_copy::<_, Vec<[Option<&mut SimChunk>; 8]>>(&ptrs))
        }
    }
    */

    // #[inline]
    // pub fn push_neighbor_sim_updates(&mut self, point: IVec3) {
    //     for y in -1..=1 {
    //         for x in -1..=1 {
    //             for z in -1..=1 {
    //                 let offset = ivec3(x, y, z);
    //                 let neighbor = point + offset;
    //                 // if self.get_voxel(neighbor).is_simulated() {
    //                 //     self.push_sim_update(neighbor);
    //                 // }
    //                 // self.updated_chunks.insert(chunk_point(neighbor));
    //                 self.push_sim_update(neighbor);
    //             }
    //         }
    //     }
    // }

    // #[inline]
    // pub fn push_sim_update(&mut self, point: IVec3) {
    //     if self.in_bounds(point) {
    //         let (chunk_index, voxel_index) =
    // Self::chunk_and_voxel_indices(point);         Self::add_update_mask(&mut
    // self.sim_updates, chunk_index, voxel_index);     }
    // }

    // #[inline]
    // pub fn add_update_mask(mask: &mut UpdateBuffer, chunk_point: ChunkPoint,
    // voxel_index: usize) {     // info!("adding update mask: {:?}",
    // (chunk_index, voxel_index));     let mask_index = voxel_index >> 6; //
    // voxel_index / 64     let bit_index = voxel_index & 63; // voxel_index %
    // 64

    //     let chunk_mask = mask.entry(chunk_point).or_insert([0u64; CHUNK_LENGTH /
    // 64]);     if cfg!(feature = "safe-bounds") {
    //         chunk_mask[mask_index] |= 1 << bit_index;
    //     } else {
    //         unsafe {
    //             // 5% faster, use when we feel fine with callers of this
    //             *chunk_mask.get_unchecked_mut(mask_index) |= 1 << bit_index;
    //         }
    //     }
    // }
}

pub struct ChunkKeys {
    pub start_chunk_point: IVec3,
    pub keys: [Option<ChunkKey>; CHUNK_VIEW_LENGTH],
}

pub struct ChunkView<'a> {
    pub start_chunk_point: IVec3,
    pub chunks: [Option<&'a mut SimChunk>; CHUNK_VIEW_LENGTH],
}

impl<'a> std::fmt::Debug for ChunkView<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChunkView")
            .field("start_chunk_point", &self.start_chunk_point).field("chunks", &self.chunks).finish()
    }
}

impl<'a> ChunkView<'a> {
    pub fn linearize_chunk(chunk_point: IVec3) -> usize {
        if chunk_point.min_element() < 0 || chunk_point.max_element() >= CHUNK_VIEW_SIZE as i32 {
            panic!("chunk point out of bounds: {:?}", chunk_point);
        }

        let IVec3 { x, y, z} = chunk_point;
        z as usize
            + x as usize * CHUNK_VIEW_SIZE
            + y as usize * CHUNK_VIEW_SIZE * CHUNK_VIEW_SIZE
    }

    pub fn delinearize_chunk(index: usize) -> IVec3 {
        assert!(index < CHUNK_VIEW_LENGTH);

        let y = index / (CHUNK_VIEW_SIZE * CHUNK_VIEW_SIZE);
        let x = (index / CHUNK_VIEW_SIZE) % CHUNK_VIEW_SIZE;
        let z = index % CHUNK_VIEW_SIZE;
        ivec3(x as i32, y as i32, z as i32)
    }

    pub fn get_voxel(&self, chunk_index: usize, voxel_index: usize) -> Option<Voxel> {
        if let Some(chunk) = &self.chunks[chunk_index] {
            Some(chunk.voxels[voxel_index])
        } else {
            None
        }
    }

    pub fn set_voxel(&mut self, chunk_index: usize, voxel_index: usize, voxel: Voxel) {
        if let Some(chunk) = &mut self.chunks[chunk_index] {
            chunk.voxels[voxel_index] = voxel;
        }
    }

    pub fn get_relative_voxel(
        &self,
        origin_chunk_index: usize,
        origin_voxel_index: usize,
        relative: IVec3,
    ) -> Option<(usize, usize, Voxel)> {
        let (chunk_index, voxel_index) =
            self.get_relative_voxel_indices(origin_chunk_index, origin_voxel_index, relative)?;
        let voxel = self.get_voxel(chunk_index, voxel_index)?;
        Some((chunk_index, voxel_index, voxel))
    }

    // Note: this will not work on relative positions larger than 16 movements
    // outside of a chunk
    pub fn get_relative_voxel_indices(
        &self,
        origin_chunk_index: usize,
        origin_voxel_index: usize,
        relative: IVec3,
    ) -> Option<(usize, usize)> {
        // happy path, we just add the stride
        let z_stride = relative.z as usize;
        let x_stride = (relative.x as usize) * CHUNK_WIDTH;
        let y_stride = (relative.y as usize) * CHUNK_WIDTH * CHUNK_WIDTH;
        let relative_stride = z_stride + x_stride + y_stride;

        let new_index = origin_voxel_index + relative_stride;
        // use underflowing to our advantage, if the stride goes under 0 it'll wrap to
        // usize::MAX and fail this.
        if new_index < CHUNK_LENGTH {
            // happy path
            return Some((origin_chunk_index, new_index));
        }

        // handle edge case
        assert!(
            relative.x.abs() < CHUNK_WIDTH as i32
                && relative.y.abs() < CHUNK_WIDTH as i32
                && relative.z.abs() < CHUNK_WIDTH as i32
        );

        let origin_point = delinearize(origin_voxel_index);
        let mut combined_point = origin_point + relative;

        let mut chunk_point = Self::delinearize_chunk(origin_chunk_index);
        chunk_point += combined_point.div_euclid(IVec3::splat(CHUNK_WIDTH as i32));
        combined_point = combined_point.rem_euclid(IVec3::splat(CHUNK_WIDTH as i32));

        if chunk_point.min_element() < 0 || chunk_point.max_element() >= CHUNK_VIEW_SIZE as i32 {
            None
        } else {
            let relative_chunk_index = Self::linearize_chunk(chunk_point);
            let relative_voxel_index = linearize(combined_point);
            Some((relative_chunk_index, relative_voxel_index))
        }
    }

    pub fn simulate(&mut self, tick: FallingSandTick) {
        // TODO: Iterate through internal voxels first to avoid accessing other chunks,
        // then iterate through each 'face', then iterate through each 'corner'.
        // This should minimize cache misses, but may cause some visible seams.

        // For now just iterate through every voxel in a linear fashion.

        // TODO: figure out update masks here
        // we can't clear voxels on the edges of chunks, because they might
        // still be able to move if the neighboring chunk was within the
        // active area. so
        for chunk_index in 0..self.chunks.len() {
            if self.chunks[chunk_index].is_none() {
                continue;
            }

            for voxel_index in 0..CHUNK_LENGTH {
                let voxel = {
                    let chunk = self.chunks[chunk_index].as_ref().unwrap();
                    let voxel_data = chunk.voxels[voxel_index];
                    voxel_data
                };

                if !voxel.is_simulated() {
                    continue;
                }

                voxel.simulate(self, chunk_index, voxel_index, tick);
            }
        }
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
    fn chunk_view() {
        let mut sim = SimChunks::new();
        sim.add_chunk(ChunkPoint(ivec3(0, 0, 0)), SimChunk::new());
        sim.add_chunk(ChunkPoint(ivec3(1, 0, 0)), SimChunk::new());
        sim.add_chunk(ChunkPoint(ivec3(1, 0, 1)), SimChunk::new());
        sim.add_chunk(ChunkPoint(ivec3(0, 0, 1)), SimChunk::new());
        sim.add_chunk(ChunkPoint(ivec3(0, 1, 0)), SimChunk::new());
        sim.add_chunk(ChunkPoint(ivec3(1, 1, 0)), SimChunk::new());
        sim.add_chunk(ChunkPoint(ivec3(1, 1, 1)), SimChunk::new());
        sim.add_chunk(ChunkPoint(ivec3(0, 1, 1)), SimChunk::new());

        let views = sim.chunk_views();
        println!("views: {:?}", views);
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
        let mut chunks = SimChunks::new();
        chunks.add_chunk(ChunkPoint(ivec3(0, 0, 0)), SimChunk::new());

        // basic set
        chunks.set_voxel(ivec3(0, 0, 0), Voxel::Dirt);
        assert_eq!(chunks.get_voxel(ivec3(0, 0, 0)), Some(Voxel::Dirt));

        // oob
        chunks.set_voxel(ivec3(-1, 0, 0), Voxel::Dirt);
        assert_eq!(chunks.get_voxel(ivec3(-1, 0, 0)), None);

        // voxel data
        chunks.set_voxel(ivec3(1, 0, 0), Voxel::Water(default()));
        assert_eq!(chunks.get_voxel(ivec3(1, 0, 0)), Some(Voxel::Water(default())));
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
