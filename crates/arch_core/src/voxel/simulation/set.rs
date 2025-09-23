//! Bitset per voxel in a chunk.

use bevy::prelude::*;

use crate::voxel::simulation::data::{CHUNK_LENGTH, linearize};

pub const SET_LEN: usize = CHUNK_LENGTH / 64;

pub struct SetReader<'a> {
    set: &'a ChunkSet,
    mask_index: usize,
    occupancy_mask: u64,
    current_mask: u64,
}

impl<'a> Iterator for SetReader<'a> {
    type Item = usize; // voxel index
    fn next(&mut self) -> Option<Self::Item> {
        while self.occupancy_mask != 0 || self.current_mask != 0 {
            if self.current_mask != 0 {
                // `bitset & -bitset` returns a bitset with only the lowest significant bit set
                let t = self.current_mask & self.current_mask.wrapping_neg();
                let trailing = self.current_mask.trailing_zeros() as usize;
                let voxel_index = self.mask_index * 64 + trailing;
                self.current_mask ^= t;
                return Some(voxel_index);
            } else {
                // `bitset & -bitset` returns a bitset with only the lowest significant bit set
                let t = self.occupancy_mask & self.occupancy_mask.wrapping_neg();
                let trailing = self.occupancy_mask.trailing_zeros() as usize;
                self.occupancy_mask ^= t;
                self.mask_index = trailing;

                self.current_mask = if cfg!(feature = "safe-bounds") {
                    self.set.set[self.mask_index]
                } else {
                    unsafe { *self.set.set.get_unchecked(self.mask_index) }
                }
            }
        }

        None
    }
}

/// Bitset over each element in a chunk.
///
/// Currently hardcoded to 16^3 chunks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub struct ChunkSet {
    /// Bitset overarching the underlying sets, a 1 bit represents that the lower level bitset
    /// has at least 1 bit set in it, while a 0 means it is empty.
    occupancy: u64,
    set: [u64; SET_LEN],
}

impl ChunkSet {
    pub fn filled() -> Self {
        Self { occupancy: u64::MAX, set: [u64::MAX; SET_LEN] }
    }

    pub fn empty() -> Self {
        Self { occupancy: 0u64, set: [0u64; SET_LEN] }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.occupancy = 0;
        for mask in &mut self.set {
            *mask = 0;
        }
    }

    #[inline]
    pub fn set(&mut self, voxel_index: usize) {
        debug_assert!(
            voxel_index < CHUNK_LENGTH,
            "tried to set a voxel index outside of the chunk: {voxel_index}"
        );
        let mask = voxel_index / 64;
        let bit = voxel_index % 64;
        self.occupancy |= 1 << mask;
        self.set[mask] |= 1 << bit;
    }

    #[inline]
    pub fn get(&self, voxel_index: usize) -> bool {
        let mask = voxel_index / 64;
        let bit = voxel_index % 64;
        (self.set[mask] & (1 << bit)) != 0
    }

    // this is self contained, doesn't need to know about surrounding Z because the boundaries
    // of the Z are contained within 16 bits.
    #[inline]
    pub fn spread_z_individual(set: u64) -> u64 {
        const LEFT_EDGE_MASK: u64 =
            0b0111111111111111_0111111111111111_0111111111111111_0111111111111111;
        const RIGHT_EDGE_MASK: u64 =
            0b1111111111111110_1111111111111110_1111111111111110_1111111111111110;
        set | ((set & LEFT_EDGE_MASK) << 1) | ((set & RIGHT_EDGE_MASK) >> 1)
    }

    #[inline]
    pub fn spread_z(&mut self) {
        for i in 0..SET_LEN {
            self.set[i] = Self::spread_z_individual(self.set[i]);
        }
    }

    #[inline]
    pub fn spread_x_individual(&self, i: usize) -> u64 {
        let before = if i > 0 {
            (self.set[i - 1]
                & 0b0000000000000000_0000000000000000_0000000000000000_1111111111111111)
                << (64 - 16)
        } else {
            0u64
        };

        let after = if i + 1 < self.set.len() {
            self.set[i + 1]
                & 0b1111111111111111_0000000000000000_0000000000000000_0000000000000000
                    >> (64 - 16)
        } else {
            0u64
        };

        self.set[i] | self.set[i] << 16 | self.set[i] >> 16 | before | after
    }

    #[inline]
    pub fn spread_x(&mut self) {
        for i in 0..self.set.len() {
            self.set[i] = self.spread_x_individual(i);
        }
    }

    #[inline]
    pub fn spread_y(&mut self) {
        self.occupancy = self.occupancy | self.occupancy << 4 | self.occupancy >> 4;

        // We have to be careful here, otherwise dirty updates will be pushed
        // across all Y levels.
        // This pushes the dirty flags to the *previous* index, so that we don't
        // keep propagating upwards, only behind us.
        for i in (4..self.set.len()).rev() {
            self.set[i] = self.set[i] | self.set[i - 4];
        }

        for i in 0..(self.set.len() - 4) {
            self.set[i] = self.set[i] | self.set[i + 4];
        }
    }

    // Here begins hell, I have manually spread modified -> dirty from neighboring chunks

    // +Y
    pub fn pull_above_chunk(&mut self, above: &ChunkSet) {
        let top_start = 0;
        let bottom_start = SET_LEN - 4;

        // 0 = (0..4, 0, 0..16)
        // 1 = (4..8, 0, 0..16)
        // 2 = (8..12, 0, 0..16)
        // 3 = (12..16, 0, 0..16)

        // 61 = (0..4, 16, 0..16)
        // 62 = (4..8, 16, 0..16)
        // 63 = (8..12, 16, 0..16)
        // 64 = (12..16, 16, 0..16)

        let top_layer = top_start..top_start + 4;
        let bottom_layer = bottom_start..bottom_start + 4;

        for (top_index, bottom_index) in top_layer.zip(bottom_layer) {
            // spread xz plane bits of the above before pulling
            let above_spread_x = above.spread_x_individual(bottom_index);
            let above_spread_xz = Self::spread_z_individual(above_spread_x);
            self.set[top_index] |= above_spread_xz;
        }
    }

    // -Y
    pub fn pull_below_chunk(&mut self, below: &ChunkSet) {
        let top_start = 0;
        let bottom_start = SET_LEN - 4;

        let top_layer = top_start..top_start + 4;
        let bottom_layer = bottom_start..bottom_start + 4;

        for (top_index, bottom_index) in top_layer.zip(bottom_layer) {
            // spread xz plane bits of the above before pulling
            let below_spread_x = below.spread_x_individual(top_index);
            let below_spread_xz = Self::spread_z_individual(below_spread_x);
            self.set[bottom_index] |= below_spread_xz;
        }
    }

    // TODO:
    // Neighbors in total: 26
    // 6 direct
    // 8 horizontal diagonal
    // 4 vertical diagonal
    // 8 corners
    // 6 + 8 + 4 + 8 = 26

    // direct: 4 u64, (16x * 16z = 256xz / 64bits = 4 u64)
    // 6
    // +Y [`ChunkSet::pull_above_chunk`]
    // -Y [`ChunkSet::pull_below_chunk`]
    // +X
    // -X
    // +Z
    // -Z

    // Probably don't do these right now, lots of work for maybe not much benefit
    // horizontal diagonals: single u64 masked to 16 bits
    // 8 variations
    // -Y -X
    // -Y -Z
    // -Y +X
    // -Y +Z
    // +Y -X
    // +Y -Z
    // +Y +X
    // +Y +Z

    // vertical diagonals: 16 u64s masked to 1 bit, probably not worthwhile
    // 4 variations
    // +X +Z
    // +X -Z
    // -X +Z
    // -X -Z

    // corners: single bit, probably not worth the reading of corner chunks
    // 8
    // +Y +X +Z
    // +Y +X -Z
    // +Y -X +Z
    // +Y -X -Z
    // -Y +X +Z
    // -Y +X -Z
    // -Y -X +Z
    // -Y -X -Z

    pub fn iter(&self) -> SetReader<'_> {
        SetReader {
            set: self,
            mask_index: 0,
            occupancy_mask: self.occupancy & !0b1, // remove the first so we don't repeat a mask
            current_mask: self.set[0],
        }
    }

    pub fn display(&self) -> String {
        let mut layers = String::new();
        for mask in self.set {
            layers += &format!("\n{:0b}", mask);
        }
        layers
    }
}

#[cfg(test)]
mod test {
    use crate::voxel::simulation::data::delinearize;

    use super::*;

    #[test]
    pub fn iter() {
        let mut set = ChunkSet::empty();
        set.set(0);
        set.set(1);
        set.set(2);
        set.set(256);
        set.set(257);
        set.set(4094);
        set.set(4095);

        let mut iter = set.iter();
        assert_eq!(iter.next(), Some(0));
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(256));
        assert_eq!(iter.next(), Some(257));
        assert_eq!(iter.next(), Some(4094));
        assert_eq!(iter.next(), Some(4095));

        for (index, item) in ChunkSet::filled().iter().enumerate() {
            assert_eq!(index, item);
        }
    }

    #[test]
    pub fn spread_z() {
        let mut set = ChunkSet::empty();

        set.set[0] = 0b1000000000000000_0100000000000001_0000000000000001_0000000000000000;
        set.spread_z();

        let should_be = 0b1100000000000000_1110000000000011_0000000000000011_0000000000000000;
        assert_eq!(set.set[0], should_be, "spread_z failed: {:b} != {:b}", set.set[0], should_be,);
    }

    #[test]
    pub fn spread_x() {
        let mut set = ChunkSet::empty();

        set.set(linearize(ivec3(1, 1, 1)));
        println!("before spread:");
        for voxel_index in set.iter() {
            println!("{:?}", voxel_index);
            println!("{:?}", delinearize(voxel_index));
        }
        set.spread_x();
        // println!("{:?}", set.set[0]);
        // println!("{:?}", set.set[4]);
        // println!("{:?}", set.set[8]);
        println!("after spread:");
        for voxel_index in set.iter() {
            println!("{:?}", voxel_index);
            println!("{:?}", delinearize(voxel_index));
        }
        // assert!(set.get(linearize(ivec3(0, 0, 0))));
        // assert!(set.get(linearize(ivec3(1, 0, 0))));
        // assert!(set.get(linearize(ivec3(2, 0, 0))));
    }

    #[test]
    pub fn spread() {
        let mut set = ChunkSet::empty();

        set.set(linearize(ivec3(1, 1, 1)));
        set.spread_z();
        set.spread_x();
        set.spread_y();
        for voxel_index in set.iter() {
            println!("{:?}", delinearize(voxel_index as usize));
        }

        println!("spread count: {:?}", set.iter().count());
    }
}
