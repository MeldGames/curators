use std::hash::{Hash, Hasher};

use bevy::platform::collections::hash_map::IterMut;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use fxhash::FxHashMap;
use slotmap::SlotMap;
#[cfg(feature = "trace")]
use tracing::*;

use crate::voxel::simulation::SimSwapBuffer;
use crate::voxel::simulation::rle::RLEChunk;
use crate::voxel::simulation::view::ChunkView;
use crate::voxel::voxel::VoxelChangeset;
use crate::voxel::{Voxel, Voxels};

pub const CHUNK_WIDTH_BITSHIFT: usize = 4;
pub const CHUNK_WIDTH_BITSHIFT_Y: usize = CHUNK_WIDTH_BITSHIFT * 2;
pub const CHUNK_REMAINDER: i32 = (CHUNK_WIDTH - 1) as i32;
pub const CHUNK_WIDTH: usize = 1 << CHUNK_WIDTH_BITSHIFT;
pub const CHUNK_LENGTH: usize = CHUNK_WIDTH * CHUNK_WIDTH * CHUNK_WIDTH;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Reflect, Deref, DerefMut, Hash)]
pub struct ChunkPoint(pub IVec3);

pub fn plugin(app: &mut App) {
    app.register_type::<SimChunk>();
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub struct SimChunk {
    pub voxel_changeset: VoxelChangeset,
    /// Voxels that are considered dirty still.
    pub dirty: [u64; CHUNK_LENGTH / 64],
    // lets try just a 4x4x4 chunk
    pub voxels: [u16; CHUNK_LENGTH],
}

impl Default for SimChunk {
    fn default() -> Self {
        Self {
            voxel_changeset: default(),
            dirty: [0u64; CHUNK_LENGTH / 64],
            voxels: [0; CHUNK_LENGTH],
        }
    }
}

impl SimChunk {
    pub fn new() -> Self {
        Self::default()
    }
}

slotmap::new_key_type! { struct ChunkKey; }

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
    ChunkPoint(point >> (CHUNK_WIDTH_BITSHIFT as i32))
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
        let chunk_key = self.chunks.insert(sim_chunk);
        self.from_chunk_point.insert(chunk_point, chunk_key);
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

        Voxel::from_data(voxel)
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
                chunk.voxels[voxel_index] = voxel.data();
            } else {
                unsafe {
                    *chunk.voxels.get_unchecked_mut(voxel_index) = voxel.data();
                }
            }

            chunk.voxel_changeset.set(voxel);

            true
        } else {
            false
        }

        // self.updated_chunks.insert(chunk_point);
        // self.push_neighbor_sim_updates(point);
    }

    /// Create a 2x2x2 area of chunks based on the current margolus offset.
    pub fn construct_blocks(&self) -> Vec<[Option<ChunkKey>; 8]> {
        use bevy::platform::collections::hash_map::Entry;

        let mut to_index: HashMap<ChunkPoint, usize> = HashMap::new();
        let mut blocks: Vec<[Option<ChunkKey>; 8]> = Vec::new();

        let offset = MARGOLUS_OFFSETS[self.margolus_offset];
        for (&chunk_point, &chunk_key) in self.from_chunk_point.iter() {
            let anchor = (*chunk_point + offset) / 2;

            // Linearize the chunk position into a 2x2x2 block (x, y, z in {0,1})
            let rel = *chunk_point - anchor * 2;
            let chunk_index =
                (rel.x as usize & 1) | ((rel.y as usize & 1) << 1) | ((rel.z as usize & 1) << 2);

            let block_index = match to_index.entry(ChunkPoint(anchor)) {
                Entry::Occupied(entry) => *entry.get(),
                Entry::Vacant(entry) => {
                    blocks.push([None; 8]);
                    let block_index = blocks.len() - 1;
                    entry.insert(block_index);
                    block_index
                },
            };

            let block = blocks.get_mut(block_index).unwrap();
            block[chunk_index] = Some(chunk_key);
        }

        blocks
    }

    /// Split the [`SimChunks`] into a list of [`ChunkView`]s that can be
    /// processed in parallel.
    pub fn chunk_views<'a>(&'a mut self) -> Vec<ChunkView<'a, 2>> {
        let blocks = self.construct_blocks();

        unsafe {
            if let Some(mut disjoint_chunks) = self.get_disjoint_blocks_mut(blocks.as_slice()) {
                disjoint_chunks
                    .iter()
                    .map(|block| ChunkView::<2> { chunks: block })
                    .collect::<Vec<_>>()
            } else {
                panic!("Some blocks were joint");
            }
        }
    }

    /// Get each individual chunk in a block as a mutable reference.
    pub unsafe fn get_disjoint_blocks_mut(
        &mut self,
        blocks: &[[Option<ChunkKey>; 8]],
    ) -> Option<Vec<[Option<&mut SimChunk>; 8]>> {
        let mut ptrs: Vec<[Option<*mut SimChunk>; 8]> = vec![[None; 8]; blocks.len()];

        // Verify chunk keys are already aliased
        let mut aliased = HashSet::new();

        unsafe {
            for block_index in 0..blocks.len() {
                for chunk_index in 0..8 {
                    if let Some(chunk_key) = blocks[block_index][chunk_index] {
                        ptrs[block_index][chunk_index] =
                            self.chunks.get_mut(chunk_key).map(|s| s as *mut SimChunk);

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
        let chunks = SimChunks::new();
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
