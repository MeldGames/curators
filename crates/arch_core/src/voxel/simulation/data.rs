use bevy::prelude::*;
#[cfg(feature = "trace")]
use tracing::*;

use crate::voxel::mesh::RenderChunks;
use crate::voxel::simulation::{RenderSwapBuffer, SimSwapBuffer};
use crate::voxel::{Voxel, Voxels};

pub const CHUNK_WIDTH_BITSHIFT: usize = 4;
pub const CHUNK_WIDTH_BITSHIFT_Y: usize = CHUNK_WIDTH_BITSHIFT * 2;
pub const CHUNK_REMAINDER: i32 = (CHUNK_WIDTH - 1) as i32;
pub const CHUNK_WIDTH: usize = 1 << CHUNK_WIDTH_BITSHIFT;
pub const CHUNK_LENGTH: usize = CHUNK_WIDTH * CHUNK_WIDTH * CHUNK_WIDTH;

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
    let render_swap_buffer = RenderSwapBuffer(voxels.sim_chunks.create_update_buffer());
    commands.entity(trigger.target()).insert((sim_swap_buffer, render_swap_buffer));
}

pub type UpdateBuffer = Vec<[u64; CHUNK_LENGTH / 64]>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub struct SimChunk {
    // lets try just a 4x4x4 chunk
    pub voxels: [u16; CHUNK_LENGTH],
}

impl SimChunk {
    pub fn new() -> Self {
        Self { voxels: [0; CHUNK_LENGTH] }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub struct SimChunks {
    pub chunks: Vec<SimChunk>,
    pub chunk_strides: [usize; 3],

    pub sim_updates: UpdateBuffer,    // bitmask of updates
    pub render_updates: UpdateBuffer, // bitmask of updates

    pub chunk_size: IVec3,
    pub voxel_size: IVec3,
}

#[inline]
pub fn to_linear_index(relative_point: IVec3) -> usize {
    debug_assert!(
        // relative_point.x < CHUNK_WIDTH as i32
        //     && relative_point.y < CHUNK_WIDTH as i32
        //     && relative_point.z < CHUNK_WIDTH as i32
        //     && relative_point.x >= 0
        //     && relative_point.y >= 0
        //     && relative_point.z >= 0
        ((relative_point.x | relative_point.y | relative_point.z) & !15) != 0
    );

    // z + x * 4 + y * 16
    // zxy order for now, maybe check if yxz is better later since the most checks
    // are vertical
    (relative_point.z + (relative_point.x << CHUNK_WIDTH_BITSHIFT) + (relative_point.y << CHUNK_WIDTH_BITSHIFT_Y)) as usize
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
    // super::morton::to_morton_index(relative_point)
}

#[inline]
pub fn delinearize(index: usize) -> IVec3 {
    from_linear_index(index)
    // super::morton::from_morton_index(index)
}

#[inline]
pub fn chunk_point(point: IVec3) -> IVec3 {
    // not euclidean (point / 16)
    point >> (CHUNK_WIDTH_BITSHIFT as i32)
}

pub struct UpdateIterator<'a> {
    pub chunk_updates: &'a mut UpdateBuffer,
    pub chunk_index: usize,
    pub mask_index: usize,
}

impl<'a> Iterator for UpdateIterator<'a> {
    type Item = (usize, usize);

    // (chunk_index, voxel_index)

    fn next(&mut self) -> Option<Self::Item> {
        const MASK_LENGTH: usize = CHUNK_LENGTH / 64;
        // println!("UPDATE ITERATOR NEXT");
        while self.mask_index < MASK_LENGTH && self.chunk_index < self.chunk_updates.len() {
            let bitset = if cfg!(feature = "safe-bounds") {
                self.chunk_updates[self.chunk_index][self.mask_index]
            } else {
                unsafe {
                    *self.chunk_updates.get_unchecked(self.chunk_index).get_unchecked(self.mask_index)
                }
            };

            if bitset != 0 {
                // `bitset & -bitset` returns a bitset with only the lowest significant bit set
                let t = bitset & bitset.wrapping_neg();
                let trailing = bitset.trailing_zeros() as usize;
                let voxel_index = self.mask_index * 64 + trailing;
                if cfg!(feature = "safe-bounds") {
                    self.chunk_updates[self.chunk_index][self.mask_index] ^= t;
                } else {
                    unsafe {
                        *self.chunk_updates.get_unchecked_mut(self.chunk_index).get_unchecked_mut(self.mask_index) ^= t;
                    }
                }
                return Some((self.chunk_index, voxel_index));
            } else {
                self.mask_index += 1;
                if self.mask_index == MASK_LENGTH {
                    self.mask_index = 0;
                    self.chunk_index += 1;
                }
            }
        }

        None
    }
}

impl SimChunks {
    pub fn new(voxel_size: IVec3) -> Self {
        let chunk_size = (voxel_size / CHUNK_WIDTH as i32) + IVec3::ONE;
        // println!("chunk_size: {:?}", chunk_size);

        // stride[0] = x;
        // stride[1] = z;
        // stride[2] = y;
        let chunk_strides = [1, chunk_size.x as usize, (chunk_size.x * chunk_size.z) as usize];
        Self {
            chunks: vec![SimChunk::new(); (chunk_size.x * chunk_size.y * chunk_size.z) as usize],
            sim_updates: Self::create_update_buffer_from_size(chunk_size),
            render_updates: Self::create_update_buffer_from_size(chunk_size),
            chunk_strides,
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
        vec![[0; CHUNK_LENGTH / 64]; (chunk_size.x * chunk_size.y * chunk_size.z) as usize]
    }

    pub fn create_update_buffer(&self) -> UpdateBuffer {
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
        // ((relative_point.x | relative_point.y | relative_point.z) & !15) != 0
    }

    #[inline]
    pub fn get_voxel_from_indices(&self, chunk_index: usize, voxel_index: usize) -> Voxel {
        let voxel = if cfg!(feature = "safe-bounds") {
            let chunk = &self.chunks[chunk_index];
            chunk.voxels[voxel_index]
        } else {
            unsafe {
                let chunk = self.chunks.get_unchecked(chunk_index);
                *chunk.voxels.get_unchecked(voxel_index)
            }
        };

        Voxel::from_data(voxel)
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
        if cfg!(feature = "safe-bounds") {
            let chunk = &mut self.chunks[chunk_index];
            chunk.voxels[voxel_index] = voxel.data();
        } else {
            unsafe {
                let chunk = self.chunks.get_unchecked_mut(chunk_index);
                *chunk.voxels.get_unchecked_mut(voxel_index) = voxel.data();
            }
        }

        Self::add_update_mask(&mut self.render_updates, chunk_index, voxel_index);
        self.push_neighbor_sim_updates(point);
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
    pub fn push_neighbor_sim_updates(&mut self, point: IVec3) {
        for y in -1..=1 {
            for x in -1..=1 {
                for z in -1..=1 {
                    let offset = ivec3(x, y, z);
                    let neighbor = point + offset;
                    self.push_sim_update(neighbor);
                }
            }
        }
    }

    #[inline]
    pub fn push_render_update(&mut self, point: IVec3) {
        if self.in_bounds(point) {
            let (chunk_index, voxel_index) = self.chunk_and_voxel_indices(point);
            Self::add_update_mask(&mut self.render_updates, chunk_index, voxel_index);
        }
    }

    #[inline]
    pub fn push_sim_update(&mut self, point: IVec3) {
        if self.in_bounds(point) {
            let (chunk_index, voxel_index) = self.chunk_and_voxel_indices(point);
            Self::add_update_mask(&mut self.sim_updates, chunk_index, voxel_index);
        }
    }

    #[inline]
    pub fn add_update_mask(mask: &mut UpdateBuffer, chunk_index: usize, voxel_index: usize) {
        // info!("adding update mask: {:?}", (chunk_index, voxel_index));
        let mask_index = voxel_index >> 6; // voxel_index / 64
        let bit_index = voxel_index & 63; // voxel_index % 64
 
        if cfg!(feature = "safe-bounds") {
            mask[chunk_index][mask_index] |= 1 << bit_index;
        } else {
            unsafe { // 5% faster, use when we feel fine with callers of this
                *mask.get_unchecked_mut(chunk_index).get_unchecked_mut(mask_index) |= 1 << bit_index;
            }
        }
    }

    pub fn clear_updates(&mut self) {
        for mask in &mut self.sim_updates {
            for bits in mask {
                *bits = 0;
            }
        }

        for mask in &mut self.render_updates {
            for bits in mask {
                *bits = 0;
            }
        }
    }

    pub fn sim_updates<'a, 'b: 'a>(
        &mut self,
        swap_buffer: &'b mut UpdateBuffer,
    ) -> UpdateIterator<'a> {
        debug_assert_eq!(self.sim_updates.len(), swap_buffer.len());
        std::mem::swap(&mut self.sim_updates, swap_buffer);
        UpdateIterator { chunk_updates: swap_buffer, chunk_index: 0, mask_index: 0 }
    }

    // separate buffer for render updates so we can accumulate over multiple frames.
    pub fn render_updates<'a, 'b: 'a>(
        &mut self,
        swap_buffer: &'b mut UpdateBuffer,
    ) -> UpdateIterator<'a> {
        debug_assert_eq!(self.render_updates.len(), swap_buffer.len());
        std::mem::swap(&mut self.render_updates, swap_buffer);
        UpdateIterator { chunk_updates: swap_buffer, chunk_index: 0, mask_index: 0 }
    }

    #[inline]
    pub fn chunk_and_voxel_indices(&self, point: IVec3) -> (usize, usize) {
        // chunk index
        let chunk_point = chunk_point(point);
        let chunk_index = self.chunk_linearize(chunk_point);

        // voxel index
        let relative_voxel_point = point - (chunk_point << (CHUNK_WIDTH_BITSHIFT as i32));
        // let voxel_index = linearize_16x16x16(relative_voxel_point);
        let voxel_index = linearize(relative_voxel_point);

        (chunk_index, voxel_index)
    }

    pub fn point_from_chunk_and_voxel_indices(
        &self,
        chunk_index: usize,
        voxel_index: usize,
    ) -> IVec3 {
        let chunk_point = self.chunk_delinearize(chunk_index);
        let relative_voxel_point = delinearize(voxel_index);
        (chunk_point << (CHUNK_WIDTH_BITSHIFT as i32)) + relative_voxel_point
    }

    pub fn propagate_sim_updates(&mut self, render_chunks: &mut RenderChunks, render_swap_buffer: &mut UpdateBuffer) {
        for (chunk_index, voxel_index) in self.render_updates(render_swap_buffer) {
            let point =
                self.point_from_chunk_and_voxel_indices(chunk_index, voxel_index);
            let voxel = self.get_voxel_from_indices(chunk_index, voxel_index);
            render_chunks.set_voxel(point, voxel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        chunks.push_neighbor_sim_updates(ivec3(0, 0, 0));
        let mut buffer = chunks.create_update_buffer();
        let updates = chunks.sim_updates(&mut buffer);
        for (chunk_index, voxel_index) in updates {
            println!("chunk_index: {}, voxel_index: {}", chunk_index, voxel_index);
        }

        SimChunks::add_update_mask(&mut chunks.sim_updates, 0, 0);
        SimChunks::add_update_mask(&mut chunks.sim_updates, 0, 100);
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
}
