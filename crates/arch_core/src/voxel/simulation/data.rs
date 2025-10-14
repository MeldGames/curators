use std::hash::{Hash, Hasher};

use bevy::platform::collections::HashMap;
use bevy::platform::collections::hash_map::Entry;
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;
use slotmap::SlotMap;
#[cfg(feature = "trace")]
use tracing::*;

use std::sync::{Mutex, Arc};

use crate::sdf::Sdf;
use crate::voxel::Voxel;
use crate::voxel::simulation::FallingSandTick;
use crate::voxel::simulation::kinds::VoxelPosition;
use crate::voxel::simulation::set::ChunkSet;
use crate::voxel::voxel::VoxelSet;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub struct SimChunk {
    // Internal reference for what chunk this is.
    pub chunk_point: ChunkPoint,

    /// Voxels we have modified this iteration.
    pub modified: ChunkSet,

    pub voxels: [Voxel; CHUNK_LENGTH],
}

impl SimChunk {
    pub fn new(chunk_point: ChunkPoint) -> Self {
        Self::fill(chunk_point, Voxel::Air)
    }

    pub fn fill(chunk_point: ChunkPoint, voxel: Voxel) -> Self {
        Self { chunk_point, modified: ChunkSet::empty(), voxels: [voxel; CHUNK_LENGTH] }
    }

    pub fn set(&mut self, voxel_index: usize, voxel: Voxel) {
        let current_voxel = if cfg!(feature = "safe-bounds") {
            self.voxels[voxel_index]
        } else {
            unsafe { *self.voxels.get_unchecked(voxel_index) }
        };

        if current_voxel != voxel {
            self.modified.set(voxel_index);

            if cfg!(feature = "safe-bounds") {
                self.voxels[voxel_index] = voxel;
            } else {
                unsafe {
                    *self.voxels.get_unchecked_mut(voxel_index) = voxel;
                }
            }
        }
    }
}

#[derive(Clone)]
struct SpreadUpdate {
    chunk_point: IVec3, chunk_key: ChunkKey, dirty_key: DirtyKey, preserve: [bool; 6],
}

slotmap::new_key_type! { pub struct ChunkKey; }
slotmap::new_key_type! { pub struct DirtyKey; }
slotmap::new_key_type! { pub struct BlockKey; }

// TODO: Double buffer this data so we can throw this on another thread,
// and we can read the current chunks from the last sim
#[derive(Component, Clone)]
pub struct SimChunks {
    pub chunks: SlotMap<ChunkKey, SimChunk>, // active chunks
    pub dirty: SlotMap<DirtyKey, ChunkSet>,

    pub from_chunk_point: HashMap<ChunkPoint, (ChunkKey, DirtyKey)>,

    // 0 => 0, 0, 0
    // 1 => 1, 1, 1
    pub to_block_index: [HashMap<ChunkPoint, BlockKey>; 8],
    pub blocks: [SlotMap<BlockKey, ChunkKeys>; 8],

    /// 0..8 offsets
    pub margolus_offset: usize,

    pub spread_list: Arc<Mutex<SpreadList>>,
}

pub struct SpreadList {
    pub spread_list: HashMap<IVec3, [bool; 6]>,
}

impl SpreadList {
    pub fn new() -> Self {
        Self {
            spread_list: HashMap::with_capacity(128),
        }
    }

    pub fn mark(&mut self, chunk_point: IVec3) {
        self.spread_list.entry(chunk_point).or_insert([false; 6]);
    }

    pub fn with_preserve(&mut self, chunk_point: IVec3, preserve: [bool; 6]) {
        self.spread_list.insert(chunk_point, preserve);
    }

    pub fn iter(&self) -> impl Iterator<Item = (IVec3, [bool; 6])> {
        self.spread_list.iter().map(|(point, preserve)| (*point, *preserve))
    }

    pub fn clear(&mut self) {
        self.spread_list.clear();
    }
}

pub struct Blocks {
    pub to_index: HashMap<ChunkPoint, usize>,
    pub keys: Vec<Option<ChunkKeys>>,
}

// TODO: Figure out best looking offsets.
// Probably keep alternating the y positions each offset, this should reduce
// artifacts from cells dropping between chunks.
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
pub const fn to_linear_index(relative_point: IVec3) -> usize {
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
pub const fn from_linear_index(index: usize) -> IVec3 {
    debug_assert!(index < CHUNK_LENGTH);

    let y = index >> CHUNK_WIDTH_BITSHIFT_Y;
    let x = (index >> CHUNK_WIDTH_BITSHIFT) & CHUNK_REMAINDER;
    let z = index & CHUNK_REMAINDER; // index % CHUNK_WIDTH 

    ivec3(x as i32, y as i32, z as i32)
}

#[inline]
pub const fn linearize(relative_point: IVec3) -> usize {
    to_linear_index(relative_point)
    // super::morton::to_morton_index_lut(relative_point)
    // unsafe { super::morton::into_morton_index_bmi2(relative_point) }
}

#[inline]
pub const fn delinearize(index: usize) -> IVec3 {
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

impl SimChunks {
    pub fn new() -> Self {
        Self {
            chunks: SlotMap::with_key(),
            dirty: SlotMap::with_key(),
            from_chunk_point: HashMap::new(),
            to_block_index: std::array::from_fn(|_| HashMap::new()),
            blocks: std::array::from_fn(|_| SlotMap::with_key()),
            margolus_offset: 0,
            spread_list: Arc::new(Mutex::new(SpreadList::new())),
        }
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

    pub fn add_chunk(&mut self, chunk_point: ChunkPoint, voxels: [Voxel; CHUNK_LENGTH]) {
        let mut existing_chunk = None;

        if let Some((chunk_key, dirty_key)) = self.from_chunk_point.get(&chunk_point) {
            if let Some(chunk) = self.chunks.get_mut(*chunk_key) {
                existing_chunk = Some(chunk);
            }
        }

        if let Some(existing_chunk) = existing_chunk {
            // info!("modifying existing");
            for (index, voxel) in voxels.into_iter().enumerate() {
                existing_chunk.set(index, voxel);
            }
        } else {
            let chunk_key = self.chunks.insert(SimChunk { chunk_point, modified: ChunkSet::filled(), voxels });
            let dirty_key = self.dirty.insert(ChunkSet::filled()); // this doesn't really matter, it'll get overwritten later
            self.from_chunk_point.insert(chunk_point, (chunk_key, dirty_key));

            // set up blocks
            for offset_index in 0..8 {
                let offset = MARGOLUS_OFFSETS[offset_index];

                let corner = ((*chunk_point + offset) / 2) * 2 - offset;

                // Linearize the chunk position into a 2x2x2 block (x, y, z in {0,1})
                let rel = *chunk_point - corner;
                // info!("anchor: {corner}, chunk_point: {chunk_point:?}, offset: {offset}");
                let chunk_index = ChunkView::linearize_chunk(rel);

                let block_key = match self.to_block_index[offset_index].entry(ChunkPoint(corner)) {
                    Entry::Occupied(entry) => *entry.get(),
                    Entry::Vacant(entry) => {
                        let block_key = self.blocks[offset_index]
                            .insert(ChunkKeys { start_chunk_point: corner, keys: [None; 8] });
                        entry.insert(block_key);
                        block_key
                    },
                };

                let block = self.blocks[offset_index].get_mut(block_key).unwrap();
                block.keys[chunk_index] = Some((chunk_key, dirty_key));
            }
        }
    }

    pub fn remove_chunk(&mut self, chunk_point: ChunkPoint) {}

    #[inline]
    pub fn chunk_key_from_point(&self, chunk_point: ChunkPoint) -> Option<(ChunkKey, DirtyKey)> {
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
        if let Some((chunk_key, _dirty_key)) = self.chunk_key_from_point(chunk_point) {
            Some(self.get_voxel_from_indices(chunk_key, voxel_index))
        } else {
            None
        }
    }

    #[inline]
    pub fn set_voxel_no_spread(&mut self, point: IVec3, voxel: Voxel) -> bool {
        let (chunk_point, voxel_index) = Self::chunk_and_voxel_indices(point);
        if let Some((chunk_key, _dirty_key)) = self.chunk_key_from_point(chunk_point) {
            let chunk = self.chunks.get_mut(chunk_key).unwrap();

            // self.spread_list.lock().unwrap().mark(*chunk_point);
            chunk.set(voxel_index, voxel);

            true
        } else {
            false
        }
    }

    #[inline]
    pub fn set_voxel(&mut self, point: IVec3, voxel: Voxel) -> bool {
        let (chunk_point, voxel_index) = Self::chunk_and_voxel_indices(point);
        if let Some((chunk_key, _dirty_key)) = self.chunk_key_from_point(chunk_point) {
            let chunk = self.chunks.get_mut(chunk_key).unwrap();

            self.spread_list.lock().unwrap().mark(*chunk_point);
            chunk.set(voxel_index, voxel);

            true
        } else {
            false
        }
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

    /// Split the [`SimChunks`] into a list of [`ChunkView`]s that can be
    /// processed in parallel.
    pub fn chunk_views<'a>(&'a mut self) -> Vec<BlockView<'a>> {
        let current_blocks = &self.blocks[self.margolus_offset];
        let mut views = current_blocks
            .iter()
            .map(|(block_key, keys)| BlockView {
                block_key: block_key,
                start_chunk_point: keys.start_chunk_point,
                chunks: ChunkView { chunks: std::array::from_fn(|_| None) },
                dirty_sets: std::array::from_fn(|_| None),
            })
            .collect::<Vec<_>>();

        // block_index, key_index, key
        let flattened = current_blocks
            .iter()
            .enumerate()
            .flat_map(|(block_index, (block_key, chunk_key))| {
                chunk_key.keys.iter().enumerate().filter_map(move |(key_index, key)| {
                    key.map(|key| (block_index, key_index, key))
                })
            })
            .collect::<Vec<_>>();

        let keys =
            flattened.iter().map(|(_, _, (key, dirty_key))| (*key, *dirty_key)).collect::<Vec<_>>();
        let disjoint = unsafe { self.get_disjoint_mut_unchecked(&keys.as_slice()) };
        for ((chunk_ref, dirty_set), (block_index, key_index, chunk_key)) in
            disjoint.into_iter().zip(flattened)
        {
            let block = &mut views[block_index];
            block.chunks.chunks[key_index] = Some(chunk_ref);
            block.dirty_sets[key_index] = Some(dirty_set);
        }

        views
    }

    // TODO: Fix this, it currently leads to a double free issue.
    /// - The caller must guarantee that all `keys` are valid and unique
    ///   (disjoint).
    pub unsafe fn get_disjoint_mut_unchecked<'a>(
        &'a mut self,
        keys: &[(ChunkKey, DirtyKey)],
    ) -> Vec<(&'a mut SimChunk, &'a ChunkSet)> {
        let mut result = Vec::with_capacity(keys.len());
        for &(key, dirty_key) in keys {
            // SAFETY: The caller must guarantee that all keys are valid and disjoint.
            let ptr = self.chunks.get_unchecked_mut(key) as *mut SimChunk;
            let set = self.dirty.get_unchecked(dirty_key);
            result.push((&mut *ptr, set));
        }
        result
    }

    // pub unsafe fn get_disjoint_blocks_mut(
    // &mut self,
    // blocks: &[ChunkKeys],
    // ) -> Option<Vec<[Option<&mut SimChunk>; CHUNK_VIEW_LENGTH]>> {
    // let mut ptrs: Vec<[Option<*mut SimChunk>; CHUNK_VIEW_LENGTH]> =
    // vec![[None; CHUNK_VIEW_LENGTH]; blocks.len()];
    //
    // Verify chunk keys are already aliased
    // let mut aliased = HashSet::new();
    //
    // Safety: Each chunk key should only be aliased once, otherwise we return early
    // and no references can be used.
    // unsafe {
    // for (block_index, block) in blocks.iter().enumerate() {
    // for (chunk_index, chunk_key) in block.keys.iter().enumerate() {
    // if let Some(chunk_key) = chunk_key {
    // ptrs[block_index][chunk_index] =
    // self.chunks.get_mut(*chunk_key).map(|s| s as *mut SimChunk);
    //
    // if aliased.contains(&chunk_key) {
    // return None;
    // } else {
    // aliased.insert(chunk_key);
    // }
    // }
    // }
    // }
    //
    // Some(core::mem::transmute_copy::<_, Vec<[Option<&mut SimChunk>; 8]>>(&ptrs))
    // }
    // }

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

    /// Spread modified updates into the dirty set for the next iteration
    pub fn spread_updates(&mut self) {
        #[cfg(feature = "trace")]
        let spread_span = info_span!("spread_updates").entered();

        let mut spread_list = self.spread_list.lock().unwrap();
        if spread_list.spread_list.len() > 0 {
            info!("spread_list len: {:?}", spread_list.spread_list.len());
        }

        for (chunk_point, preserve) in spread_list.spread_list.drain() {
            let Some((chunk_key, dirty_key)) = self.from_chunk_point.get(&ChunkPoint(chunk_point)) else {
                warn!("missing from_chunk_point entry for a spread list entry: {:?}", chunk_point);
                continue;
            };

            let chunk = self.chunks.get(*chunk_key).unwrap();
            let dirty = self.dirty.get_mut(*dirty_key).unwrap();

            #[cfg(feature = "trace")]
            let span = info_span!("previous_dirty").entered();
            let previous_dirty = dirty.clone();
            #[cfg(feature = "trace")]
            span.exit();

            #[cfg(feature = "trace")]
            let span = info_span!("dirty_copy_modified").entered();
            if chunk.modified.any_set() {
                for (index, modified_mask) in chunk.modified.iter_masks().enumerate() {
                    dirty.set_mask(index, modified_mask);
                }
            } else {
                dirty.clear();
            }
            #[cfg(feature = "trace")]
            span.exit();

            // spread internally
            if dirty.any_set() {
                #[cfg(feature = "trace")]
                let span = info_span!("dirty_spread_internally").entered();

                dirty.spread_y();
                dirty.spread_x();
                dirty.spread_z();
            }

            // spread in between surrounding chunks
            // these need to be pushing not pulling
            // otherwise we will miss some updates
            if let Some((above, _)) =
                self.from_chunk_point.get(&ChunkPoint(chunk_point + IVec3::new(0, 1, 0)))
            {
                let above_chunk = self.chunks.get(*above).unwrap();
                dirty.pull_above_chunk(&above_chunk.modified);
            }

            if let Some((below, _)) =
                self.from_chunk_point.get(&ChunkPoint(chunk_point - IVec3::new(0, 1, 0)))
            {
                let below_chunk = self.chunks.get(*below).unwrap();
                dirty.pull_below_chunk(&below_chunk.modified);
            }
            // dirty.assert_occupancy("pulling vertical chunks");

            continue;

            // preserve previous dirty on boundaries
            let [above, below, right, left, back, forward] = preserve;

            // Vertical masks are fully populated on the XZ plane, so all bits are set.
            pub const VERTICAL_PRESERVE_MASK: u64 = u64::MAX;
            // Left = (x == 0)
            // Right = (x == 15)
            // there are 16 bits per X axis, so every 4 masks is a new X.
            pub const LEFT_PRESERVE_MASK: u64 =
                0b1111111111111111_0000000000000000_0000000000000000_0000000000000000;
            pub const RIGHT_PRESERVE_MASK: u64 =
                0b0000000000000000_0000000000000000_0000000000000000_1111111111111111;

            // Trickier one, the edge on the Z axis is the first or last bit of every 16
            // bits.
            pub const FORWARD_PRESERVE_MASK: u64 =
                0b1000000000000000_1000000000000000_1000000000000000_1000000000000000;
            pub const BACKWARD_PRESERVE_MASK: u64 =
                0b0000000000000001_0000000000000001_0000000000000001_0000000000000001;

            // For every index, we can potentially preserve the start and end in the Z
            // directions.
            let mut initial_preserve_mask = 0u64;
            // if !forward {
            //     initial_preserve_mask |= FORWARD_PRESERVE_MASK;
            // }
            // if !back {
            //     initial_preserve_mask |= BACKWARD_PRESERVE_MASK;
            // }

            for (index, modified_mask) in chunk.modified.iter_masks().enumerate() {
                use crate::voxel::simulation::set::{
                    is_bottom_index, is_left_index, is_right_index, is_top_index,
                };

                // we preserve some dirty bits from the last frame in the case the voxel had no
                // viable neighbor next to it. this could've changed by
                // adding chunks to the simulation, or more commonly, the margolus offset
                // changing
                let mut preserve_mask = initial_preserve_mask;
                // if (!above && is_top_index(index)) || (!below && is_bottom_index(index)) {
                //     preserve_mask |= VERTICAL_PRESERVE_MASK;
                // }

                // if !left && is_left_index(index) {
                //     preserve_mask |= LEFT_PRESERVE_MASK;
                // } else if !right && is_right_index(index) {
                //     preserve_mask |= RIGHT_PRESERVE_MASK;
                // }

                // cool debug visualization of the preserve mask ngl
                // println!("preserve mask: {:b}", preserve_mask);

                let new_dirty =
                    dirty.get_mask(index) | (previous_dirty.get_mask(index) & preserve_mask);
                dirty.set_mask(index, new_dirty);

                // dirty.assert_occupancy("preserve masking");
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChunkKeys {
    pub start_chunk_point: IVec3,
    pub keys: [Option<(ChunkKey, DirtyKey)>; CHUNK_VIEW_LENGTH],
}

pub struct BlockView<'a> {
    pub block_key: BlockKey, // key for this block
    pub start_chunk_point: IVec3,
    // chunk, dirty set
    chunks: ChunkView<'a>,
    dirty_sets: [Option<&'a ChunkSet>; CHUNK_VIEW_LENGTH],
}

impl<'a> BlockView<'a> {
    pub fn simulate(&mut self, spread_list: Arc<Mutex<SpreadList>>, tick: FallingSandTick) {
        // TODO: Iterate through internal voxels first to avoid accessing other chunks,
        // then iterate through each 'face', then iterate through each 'corner'.
        // This should minimize cache misses, but may cause some visible seams.
        // Profile this, it might not be worthwhile.

        // For now just iterate through every voxel in a linear fashion.

        for chunk in self.chunks.chunks.iter_mut() {
            if let Some(chunk) = chunk {
                chunk.modified.clear();
            }
        }

        // TODO: figure out update masks here
        // we can't clear voxels on the edges of chunks, because they might
        // still be able to move if the neighboring chunk was within the
        // active area. so
        for chunk_index in 0..CHUNK_VIEW_LENGTH {
            if self.chunks.chunks[chunk_index].is_none() {
                continue;
            }

            let Some(dirty) = self.dirty_sets[chunk_index] else {
                continue;
            };

            for voxel_index in dirty.iter() {
                let voxel = {
                    let chunk = self.chunks.chunks[chunk_index].as_ref().unwrap();
                    let voxel_data = chunk.voxels[voxel_index];
                    voxel_data
                };

                if !voxel.is_simulated() {
                    continue;
                }

                let position = VoxelPosition::from_indices(chunk_index, voxel_index);
                voxel.simulate(&mut self.chunks, position, tick);
            }

            if self.chunks.chunks[chunk_index].as_ref().unwrap().modified.any_set() || dirty.any_set() {
                let rel_chunk_point = ChunkView::delinearize_chunk(chunk_index);
                let chunk_point = self.start_chunk_point + rel_chunk_point;
                let neighbors = [
                    rel_chunk_point + IVec3::Y,
                    rel_chunk_point - IVec3::Y,
                    rel_chunk_point + IVec3::X,
                    rel_chunk_point - IVec3::X,
                    rel_chunk_point + IVec3::Z,
                    rel_chunk_point - IVec3::Z,
                ];
                let exists: [bool; 6] = neighbors.map(|neighbor| {
                    if neighbor.min_element() < 0 || neighbor.max_element() > 1 {
                        false
                    } else {
                        let neighbor_index = ChunkView::linearize_chunk(neighbor);
                        self.chunks.chunks[neighbor_index].is_some()
                    }
                });

                // We should spread the modified
                let mut spread_list = spread_list.lock().unwrap();
                spread_list.with_preserve(chunk_point, exists.map(|e| !e));
            }
        }
    }
}

pub struct ChunkView<'a> {
    pub chunks: [Option<&'a mut SimChunk>; CHUNK_VIEW_LENGTH],
}

impl<'a> ChunkView<'a> {
    pub fn linearize_chunk(chunk_point: IVec3) -> usize {
        if chunk_point.min_element() < 0 || chunk_point.max_element() >= CHUNK_VIEW_SIZE as i32 {
            panic!("chunk point out of bounds: {:?}", chunk_point);
        }

        let IVec3 { x, y, z } = chunk_point;
        z as usize + x as usize * CHUNK_VIEW_SIZE + y as usize * CHUNK_VIEW_SIZE * CHUNK_VIEW_SIZE
    }

    pub fn delinearize_chunk(index: usize) -> IVec3 {
        assert!(index < CHUNK_VIEW_LENGTH);

        let y = index / (CHUNK_VIEW_SIZE * CHUNK_VIEW_SIZE);
        let x = (index / CHUNK_VIEW_SIZE) % CHUNK_VIEW_SIZE;
        let z = index % CHUNK_VIEW_SIZE;
        ivec3(x as i32, y as i32, z as i32)
    }

    pub fn get_voxel(&self, voxel_position: VoxelPosition) -> Option<Voxel> {
        if let Some(chunk) = &self.chunks[voxel_position.chunk_index] {
            // Some(chunk.voxels[voxel_position.voxel_index])
            Some(unsafe { *chunk.voxels.get_unchecked(voxel_position.voxel_index) })
        } else {
            None
        }
    }

    pub fn set_voxel(&mut self, voxel_position: VoxelPosition, voxel: Voxel) {
        if let Some(chunk) = &mut self.chunks[voxel_position.chunk_index] {
            chunk.set(voxel_position.voxel_index, voxel)
        }
    }

    pub fn get_relative_voxel(
        &self,
        voxel_position: VoxelPosition,
        relative: IVec3,
    ) -> Option<(VoxelPosition, Voxel)> {
        let relative_position = self.get_relative_voxel_position(voxel_position, relative)?;
        let voxel = self.get_voxel(relative_position)?;
        Some((relative_position, voxel))
    }

    // Note: this will not work on relative positions larger than 16 movements
    // outside of a chunk
    pub fn get_relative_voxel_position(
        &self,
        position: VoxelPosition,
        relative: IVec3,
    ) -> Option<VoxelPosition> {
        // handle edge case
        assert!(
            relative.x.abs() < CHUNK_WIDTH as i32
                && relative.y.abs() < CHUNK_WIDTH as i32
                && relative.z.abs() < CHUNK_WIDTH as i32
        );

        let mut combined_point = position.voxel_point + relative;

        let chunk_point =
            position.chunk_point + combined_point.div_euclid(IVec3::splat(CHUNK_WIDTH as i32));
        combined_point = combined_point.rem_euclid(IVec3::splat(CHUNK_WIDTH as i32));

        if chunk_point.min_element() < 0 || chunk_point.max_element() >= CHUNK_VIEW_SIZE as i32 {
            None
        } else {
            Some(VoxelPosition::from_points(chunk_point, combined_point))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linearize_delinearize_sanity() {
        let tests = [
            ivec3(0, 0, 0),
            IVec3::splat(CHUNK_WIDTH as i32 - 1),
            ivec3(1, 1, 1),
            ivec3(1, 2, 1),
            ivec3(0, 0, 1),
            ivec3(0, 1, 0),
            ivec3(1, 0, 0),
        ];

        for test in tests {
            assert_eq!(test, delinearize(linearize(test)));
        }
    }

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
        sim.add_chunk(ChunkPoint(ivec3(0, 0, 0)), [Voxel::Air; CHUNK_LENGTH]);
        sim.add_chunk(ChunkPoint(ivec3(1, 0, 0)), [Voxel::Air; CHUNK_LENGTH]);
        sim.add_chunk(ChunkPoint(ivec3(1, 0, 1)), [Voxel::Air; CHUNK_LENGTH]);
        sim.add_chunk(ChunkPoint(ivec3(0, 0, 1)), [Voxel::Air; CHUNK_LENGTH]);
        sim.add_chunk(ChunkPoint(ivec3(0, 1, 0)), [Voxel::Air; CHUNK_LENGTH]);
        sim.add_chunk(ChunkPoint(ivec3(1, 1, 0)), [Voxel::Air; CHUNK_LENGTH]);
        sim.add_chunk(ChunkPoint(ivec3(1, 1, 1)), [Voxel::Air; CHUNK_LENGTH]);
        sim.add_chunk(ChunkPoint(ivec3(0, 1, 1)), [Voxel::Air; CHUNK_LENGTH]);

        let views = sim.chunk_views();
        // println!("views: {:?}", views);
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
        chunks.add_chunk(ChunkPoint(ivec3(0, 0, 0)), [Voxel::Air; CHUNK_LENGTH]);

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
