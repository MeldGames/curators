use bevy::prelude::*;
#[cfg(feature = "trace")]
use tracing::*;

use crate::voxel::simulation::{RenderSwapBuffer, SimSwapBuffer};
use crate::voxel::{Voxel, Voxels};

pub const BLOCK_VOXELS_BITSHIFT: i32 = 2; // 4 voxels
pub const BLOCK_VOXELS: i32 = 1 << BLOCK_VOXELS_BITSHIFT;
pub const BLOCK_STRIDE: i32 = BLOCK_VOXELS * BLOCK_VOXELS * BLOCK_VOXELS;

pub const CHUNK_BLOCKS_BITSHIFT: i32 = 2; // 4;
pub const CHUNK_BLOCKS: i32 = 1 << CHUNK_BLOCKS_BITSHIFT; // 4 blocks per chunk
pub const CHUNK_VOXELS_BITSHIFT: i32 = BLOCK_VOXELS_BITSHIFT * CHUNK_BLOCKS_BITSHIFT;
pub const CHUNK_VOXELS: i32 = 1 << CHUNK_VOXELS_BITSHIFT;
pub const CHUNK_STRIDE: i32 = CHUNK_VOXELS * CHUNK_VOXELS * CHUNK_VOXELS;

pub fn plugin(app: &mut App) {
    app.register_type::<SimChunk>();
    // app.add_systems(Update, falling_sands);

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
    // let sim_swap_buffer = SimSwapBuffer(voxels.sim_chunks.create_update_buffer());
    // let render_swap_buffer = RenderSwapBuffer(voxels.sim_chunks.create_update_buffer());
    let sim_swap_buffer = SimSwapBuffer(Vec::new());
    let render_swap_buffer = RenderSwapBuffer(Vec::new());
    commands.entity(trigger.target()).insert((sim_swap_buffer, render_swap_buffer));
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub struct SimChunk {
    pub voxels: [u16; CHUNK_STRIDE as usize], // 4x4x4 voxels in blocks, 4x4x4 blocks in chunk

    pub sim_updates: [u64; 64],    // bitmask of updates
    pub render_updates: [u64; 64], // bitmask of updates
}

impl SimChunk {
    pub fn new() -> Self {
        Self { voxels: [0; CHUNK_STRIDE as usize], sim_updates: [0; 64], render_updates: [0; 64] }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub struct SimChunks {
    pub chunks: Vec<SimChunk>,
    pub chunk_strides: [usize; 3],

    pub chunk_size: IVec3,
    pub voxel_size: IVec3,
}

// | |-| | // basic layout of memory
// | | | |
// | | | |
// |-| |-|

// pub fn voxel_index(point: IVec3) -> usize {
//     let chunk_point = chunk_point(point); // chunks are every 16 voxels
//     // let chunk_index = ... need chunk_strides to get this
//     let block_point = point >> 2; // blocks are every 4 voxels
//     let relative_block_point = block_point - chunk_point << 2;
// }

#[inline]
pub fn block_relative_to_chunk(chunk_point: IVec3, point: IVec3) -> IVec3 {
    // not euclidean (point / 4)
    point - (chunk_point << CHUNK_BLOCKS_BITSHIFT)
}

#[inline]
pub fn point_relative_to_block(block_point: IVec3, point: IVec3) -> IVec3 {
    // (point - block_point * 4)
    point - (block_point << BLOCK_VOXELS_BITSHIFT)
}

#[inline]
pub fn linearize_4x4x4(relative_point: IVec3) -> usize {
    debug_assert!(
        relative_point.x < 4
            && relative_point.y < 4
            && relative_point.z < 4
            && relative_point.x >= 0
            && relative_point.y >= 0
            && relative_point.z >= 0
    );

    // z + x * 4 + y * 16
    // zxy order for now, maybe check if yxz is better later since the most checks
    // are vertical
    (relative_point.z + (relative_point.x << 2) + (relative_point.y << 4)) as usize
}

#[inline]
pub fn delinearize_4x4x4(index: usize) -> IVec3 {
    let mut index = index as i32;
    debug_assert!(index < 64);

    let y = index >> 4;
    index -= y << 4;
    let x = index >> 2;
    let z = index & 3; // index % 4
    ivec3(x, y, z)
}

#[inline]
pub fn linearize_16x16x16(relative_point: IVec3) -> usize {
    debug_assert!(
        relative_point.x < 16
            && relative_point.y < 16
            && relative_point.z < 16
            && relative_point.x >= 0
            && relative_point.y >= 0
            && relative_point.z >= 0
    );

    // z + x * 4 + y * 16
    // zxy order for now, maybe check if yxz is better later since the most checks
    // are vertical
    (relative_point.z + (relative_point.x << 4) + (relative_point.y << 8)) as usize
}

#[inline]
pub fn delinearize_16x16x16(index: usize) -> IVec3 {
    let mut index = index as i32;
    debug_assert!(index < 4096);

    let y = index >> 8;
    index -= y << 8;
    let x = index >> 4;
    let z = index & 15; // index % 4
    ivec3(x, y, z)
}

#[inline]
pub fn chunk_point(point: IVec3) -> IVec3 {
    // not euclidean (point / 16)
    point >> CHUNK_VOXELS_BITSHIFT
}

#[inline]
pub fn block_point(point: IVec3) -> IVec3 {
    // not euclidean (point / 4)
    point >> BLOCK_VOXELS_BITSHIFT
}

/// Chain a bunch of chunk update iterators together.
pub struct ChainChunkUpdateIterator<'a> {
    pub chunk_updates: &'a mut Vec<ChunkUpdateIterator>, // re-use buffer because allocations suck
    pub chunk_index: usize,
}

impl<'a> Iterator for ChainChunkUpdateIterator<'a> {
    type Item = (usize, usize); // chunk_index, voxel_index

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.chunk_index >= self.chunk_updates.len() {
                return None;
            }

            let chunk_update = &mut self.chunk_updates[self.chunk_index];
            if let Some(voxel_index) = chunk_update.next() {
                return Some((self.chunk_index, voxel_index));
            }

            self.chunk_index += 1;
        }
    }
}

pub struct ChunkUpdateIterator {
    pub chunk_updates: [u64; 64],
    pub mask_index: usize,
}

impl Iterator for ChunkUpdateIterator {
    type Item = usize; // voxel_index

    // (chunk_index, voxel_index)

    fn next(&mut self) -> Option<Self::Item> {
        while self.mask_index < 64 {
            let bitset = self.chunk_updates[self.mask_index];
            if bitset != 0 {
                // `bitset & -bitset` returns a bitset with only the lowest significant bit set
                let t = bitset & bitset.wrapping_neg();
                let trailing = bitset.trailing_zeros() as usize;
                let voxel_index = self.mask_index * 64 + trailing;
                self.chunk_updates[self.mask_index] ^= t;
                return Some(voxel_index);
            } else {
                self.mask_index += 1;
            }
        }

        None
    }
}

impl SimChunks {
    pub fn new(voxel_size: IVec3) -> Self {
        let chunk_size = (voxel_size / IVec3::splat(16)) + IVec3::ONE;
        // println!("chunk_size: {:?}", chunk_size);

        // stride[0] = x;
        // stride[1] = z;
        // stride[2] = y;
        let chunk_strides = [1, chunk_size.x as usize, (chunk_size.x * chunk_size.z) as usize];
        Self {
            chunks: vec![SimChunk::new(); (chunk_size.x * chunk_size.y * chunk_size.z) as usize],
            chunk_strides,
            chunk_size,
            voxel_size,
        }
    }

    pub fn create_update_buffer_from_size(chunk_size: IVec3) -> Vec<[u64; 64]> {
        info!(
            "creating update buffer from size: {:?} -> {}",
            chunk_size,
            (chunk_size.x * chunk_size.y * chunk_size.z) as usize
        );
        vec![[0; 64]; (chunk_size.x * chunk_size.y * chunk_size.z) as usize]
    }

    pub fn create_update_buffer(&self) -> Vec<[u64; 64]> {
        Self::create_update_buffer_from_size(self.chunk_size)
    }

    #[inline]
    pub fn chunk_linearize(&self, chunk_point: IVec3) -> usize {
        chunk_point.x as usize
            + chunk_point.z as usize * self.chunk_strides[1]
            + chunk_point.y as usize * self.chunk_strides[2]
    }

    #[inline]
    pub fn chunk_delinearize(&self, mut chunk_index: usize) -> IVec3 {
        let y = chunk_index / self.chunk_strides[2];
        chunk_index -= y * self.chunk_strides[2];
        let z = chunk_index / self.chunk_strides[1];
        let x = chunk_index % self.chunk_strides[1];
        ivec3(x as i32, y as i32, z as i32)
    }

    #[inline]
    pub fn in_bounds(&self, point: IVec3) -> bool {
        point.x >= 0
            && point.y >= 0
            && point.z >= 0
            && point.x < self.voxel_size.x
            && point.y < self.voxel_size.y
            && point.z < self.voxel_size.z
    }

    // pub fn linearize(&self, point: IVec3) -> LinearizedVoxelPoint {
    //     let chunk_point = chunk_point(point);
    //     let chunk_index = self.chunk_index(chunk_point);
    //     let block_point = block_point(point);
    //     let block_index = block_index(point_relative_to_block(block_point,
    // point));     let voxel_index = voxel_index(point);
    // }

    #[inline]
    pub fn get_voxel_from_indices(&self, chunk_index: usize, voxel_index: usize) -> Voxel {
        Voxel::from_data(self.chunks[chunk_index].voxels[voxel_index])
    }

    #[inline]
    pub fn get_voxel(&self, point: IVec3) -> Voxel {
        if !self.in_bounds(point) {
            return Voxel::Barrier;
        }

        let (chunk_index, voxel_index) = self.chunk_and_voxel_indices(point);
        self.get_voxel_from_indices(chunk_index, voxel_index)
    }

    #[inline]
    pub fn set_voxel(&mut self, point: IVec3, voxel: Voxel) {
        if !self.in_bounds(point) {
            return;
        }

        let (chunk_index, voxel_index) = self.chunk_and_voxel_indices(point);
        let chunk = &mut self.chunks[chunk_index];
        chunk.voxels[voxel_index] = voxel.data();
        self.push_neighbor_updates(point);
    }

    #[inline]
    pub fn set_voxel_no_updates(&mut self, point: IVec3, voxel: Voxel) {
        if !self.in_bounds(point) {
            return;
        }

        let (chunk_index, voxel_index) = self.chunk_and_voxel_indices(point);
        let chunk = &mut self.chunks[chunk_index];
        chunk.voxels[voxel_index] = voxel.data();
    }

    #[inline]
    pub fn push_neighbor_updates(&mut self, point: IVec3) {
        for y in -1..=1 {
            for x in -1..=1 {
                for z in -1..=1 {
                    let offset = ivec3(x, y, z);
                    let neighbor = point + offset;
                    self.push_point_update(neighbor);
                }
            }
        }
    }

    #[inline]
    pub fn push_point_update(&mut self, point: IVec3) {
        if self.in_bounds(point) {
            let (chunk_index, voxel_index) = self.chunk_and_voxel_indices(point);
            // info!("adding update point: {:?}", point);
            self.add_update_mask(chunk_index, voxel_index);
        }
    }

    #[inline]
    pub fn add_update_mask(&mut self, chunk_index: usize, voxel_index: usize) {
        // info!("adding update mask: {:?}", (chunk_index, voxel_index));
        let mask_index = voxel_index >> 6; // voxel_index / 64
        let bit_index = voxel_index & 63; // voxel_index % 64
        self.chunks[chunk_index].sim_updates[mask_index] |= 1 << bit_index;
        self.chunks[chunk_index].render_updates[mask_index] |= 1 << bit_index;
    }

    pub fn clear_updates(&mut self) {
        for chunk in self.chunks.iter_mut() {
            for mask in &mut chunk.sim_updates {
                *mask = 0;
            }
            for mask in &mut chunk.render_updates {
                *mask = 0;
            }
        }
    }

    pub fn sim_updates<'a, 'b>(
        &'a mut self,
        buffer: &'b mut Vec<ChunkUpdateIterator>,
    ) -> ChainChunkUpdateIterator<'b> {
        // debug_assert_eq!(self.sim_updates.len(), swap_buffer.len());
        for chunk in self.chunks.iter_mut() {
            let mut updates = [0; 64];
            std::mem::swap(&mut chunk.sim_updates, &mut updates);
            buffer.push(ChunkUpdateIterator { chunk_updates: updates, mask_index: 0 });
        }
        ChainChunkUpdateIterator { chunk_updates: buffer, chunk_index: 0 }
    }

    // separate buffer for render updates so we can accumulate over multiple frames.
    pub fn render_updates<'a, 'b>(
        &'a mut self,
        buffer: &'b mut Vec<ChunkUpdateIterator>,
    ) -> ChainChunkUpdateIterator<'b> {
        // debug_assert_eq!(self.sim_updates.len(), swap_buffer.len());
        for chunk in self.chunks.iter_mut() {
            let mut updates = [0; 64];
            std::mem::swap(&mut chunk.render_updates, &mut updates);
            buffer.push(ChunkUpdateIterator { chunk_updates: updates, mask_index: 0 });
        }
        ChainChunkUpdateIterator { chunk_updates: buffer, chunk_index: 0 }
    }

    #[inline]
    pub fn chunk_and_voxel_indices(&self, point: IVec3) -> (usize, usize) {
        // chunk index
        let chunk_point = chunk_point(point);
        let chunk_index = self.chunk_linearize(chunk_point);

        // voxel index
        let relative_voxel_point = point - (chunk_point << 4);
        let voxel_index = linearize_16x16x16(relative_voxel_point);

        (chunk_index, voxel_index)
    }

    pub fn point_from_chunk_and_voxel_indices(
        &self,
        chunk_index: usize,
        voxel_index: usize,
    ) -> IVec3 {
        let chunk_point = self.chunk_delinearize(chunk_index);
        let relative_voxel_point = delinearize_16x16x16(voxel_index);
        (chunk_point << 4) + relative_voxel_point
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linearize() {
        let point = ivec3(32, 3, 4);
        let block_point = point >> 2;
        let chunk_point = point >> 4;
        // println!("block_point: {:?}, chunk_point: {:?}", block_point, chunk_point);
        // println!(
        //     "block_point as voxel: {:?}, chunk_point as block: {:?}",
        //     block_point << 2,
        //     chunk_point << 2
        // );
        let relative_block_to_chunk = block_point - (chunk_point << 2);
        let relative_voxel_to_block = point - (block_point << 2);
        // println!(
        //     "relative_block_to_chunk: {:?}, relative_voxel_to_block: {:?}",
        //     relative_block_to_chunk, relative_voxel_to_block
        // );

        assert_eq!(linearize_4x4x4(ivec3(0, 0, 0)), 0);
        assert_eq!(linearize_4x4x4(ivec3(0, 0, 1)), 1);
        assert_eq!(linearize_4x4x4(ivec3(1, 0, 0)), 4);
        assert_eq!(linearize_4x4x4(ivec3(0, 1, 0)), 16);

        assert_eq!(delinearize_4x4x4(0), ivec3(0, 0, 0));
        assert_eq!(delinearize_4x4x4(1), ivec3(0, 0, 1));
        assert_eq!(delinearize_4x4x4(4), ivec3(1, 0, 0));
        assert_eq!(delinearize_4x4x4(16), ivec3(0, 1, 0));
        assert_eq!(delinearize_4x4x4(16), ivec3(0, 1, 0));

        assert_eq!(delinearize_4x4x4(linearize_4x4x4(ivec3(0, 0, 0))), ivec3(0, 0, 0));
        assert_eq!(delinearize_4x4x4(linearize_4x4x4(ivec3(1, 1, 1))), ivec3(1, 1, 1));
        assert_eq!(delinearize_4x4x4(linearize_4x4x4(ivec3(3, 0, 0))), ivec3(3, 0, 0));
        assert_eq!(delinearize_4x4x4(linearize_4x4x4(ivec3(0, 3, 0))), ivec3(0, 3, 0));
        assert_eq!(delinearize_4x4x4(linearize_4x4x4(ivec3(0, 0, 3))), ivec3(0, 0, 3));
    }

    #[test]
    fn chunk_linearize() {
        let chunks = SimChunks::new(ivec3(64, 64, 64));
        assert_eq!(chunks.chunk_delinearize(0), ivec3(0, 0, 0));
        assert_eq!(
            chunks.chunk_delinearize(chunks.chunk_linearize(ivec3(0, 0, 0))),
            ivec3(0, 0, 0)
        );
        assert_eq!(
            chunks.chunk_delinearize(chunks.chunk_linearize(ivec3(1, 1, 1))),
            ivec3(1, 1, 1)
        );
    }

    #[test]
    fn chunk_voxel_indices() {
        let chunks = SimChunks::new(ivec3(64, 64, 64));
        // chunks.chunk_and_voxel_indices(ivec3(12, 12, 12));
        // assert_eq!(chunks.chunk_and_voxel_indices(ivec3(0, 0, 0)), (0, 0));
        // assert_eq!(chunks.chunk_and_voxel_indices(ivec3(16, 0, 0)), (1, 0));
        // assert_eq!(chunks.chunk_and_voxel_indices(ivec3(16, 0, 5)), (1, 0));

        let sanity = |p: IVec3| -> IVec3 {
            let (chunk_index, voxel_index) = chunks.chunk_and_voxel_indices(p);
            chunks.point_from_chunk_and_voxel_indices(chunk_index, voxel_index)
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
        chunks.set_voxel(ivec3(1, 0, 0), Voxel::Water { lateral_energy: 2 });
        assert_eq!(chunks.get_voxel(ivec3(1, 0, 0)), Voxel::Water { lateral_energy: 2 });
    }

    #[test]
    fn update_iterator() {
        let mut chunks = SimChunks::new(ivec3(32, 32, 32));
        chunks.push_neighbor_updates(ivec3(0, 0, 0));
        let mut buffer = Vec::new();
        let updates = chunks.sim_updates(&mut buffer);
        for (chunk_index, voxel_index) in updates {
            println!("chunk_index: {}, voxel_index: {}", chunk_index, voxel_index);
        }

        chunks.add_update_mask(0, 0);
        chunks.add_update_mask(0, 100);
        buffer.clear();
        let updates = chunks.sim_updates(&mut buffer);
        println!("second round");
        for (chunk_index, voxel_index) in updates {
            println!("chunk_index: {}, voxel_index: {}", chunk_index, voxel_index);
        }

        println!("renderin round");
        for (chunk_index, voxel_index) in chunks.render_updates(&mut buffer) {
            println!("chunk_index: {}, voxel_index: {}", chunk_index, voxel_index);
        }
        // assert_eq!(updates.next(), Some((0, 0)));
        // assert_eq!(updates.next(), None);
    }

    #[test]
    fn test_shift() {
        // reference so i stop fucking up directions of bitshifts lmao
        let x = 2;
        println!("{}", x << 1); // multiply by 2
        println!("{}", x >> 1); // divide by 2
    }

    // #[test]
    // fn test_neighbor_offsets() {
    //     let rolled = (-1..=1)
    //         .flat_map(move |y| (-1..=1).flat_map(move |x| (-1..=1).map(move |z| ivec3(x, y, z))));

    //     // println!("pub const NEIGHBOR_OFFSETS: [IVec3; 27] = [");
    //     for (index, (rolled, unrolled)) in
    //         rolled.clone().zip(SimChunks::NEIGHBOR_OFFSETS).enumerate()
    //     {
    //         // println!("    ivec3({}, {}, {}),", rolled.x, rolled.y, rolled.z);
    //         // println!("    ivec3({}, {}, {}),", rolled.x, rolled.y, rolled.z);

    //         println!(
    //             "let point{} = point + ivec3({}, {}, {});",
    //             index, rolled.x, rolled.y, rolled.z
    //         );
    //         assert_eq!(rolled, unrolled);
    //     }
    //     for (index, (rolled, unrolled)) in rolled.zip(SimChunks::NEIGHBOR_OFFSETS).enumerate() {
    //         println!("self.add_update_point(point{});", index,);
    //     }
    //     // println!("];");
    // }
}
