use bevy::prelude::*;

// Morton Curve (Z-Order) implementation for 3D points in Rust
// Range: 0..16 (exclusive) for x, y, z coordinates

/// Spreads bits of a number by inserting two zeros between each bit
/// Used to prepare coordinates for Morton encoding
#[inline]
const fn spread_bits(mut value: usize) -> usize {
    // Spread the 4 bits across 12 bits with 2 zeros between each bit
    // 0000abcd -> 00a00b00c00d
    value = (value | (value << 8)) & 0x00F00F; // 0000abcd -> 0000ab0000cd
    value = (value | (value << 4)) & 0x0C30C3; // 0000ab0000cd -> 00a0b00c0d
    value = (value | (value << 2)) & 0x249249; // 00a0b00c0d -> 0a0b0c0d

    value
}

#[inline]
const fn compact_bits(mut value: usize) -> usize {
    value &= 0x249249;
    value = (value | (value >> 2)) & 0x0C30C3;
    value = (value | (value >> 4)) & 0x00F00F;
    value = (value | (value >> 8)) & 0x00000F;

    value
}

/// Converts 3D coordinates to Morton index (linearization)
#[inline]
pub const fn to_morton_index_shift_and(point: IVec3) -> usize {
    // Interleave bits: z gets the highest bits, then y, then x
    spread_bits(point.x as usize)
        | (spread_bits(point.y as usize) << 1)
        | (spread_bits(point.z as usize) << 2)
}

#[inline]
pub const fn from_morton_index(index: usize) -> IVec3 {
    // Extract interleaved bits for each coordinate
    let x = compact_bits(index); // Extract every 3rd bit starting from bit 0
    let y = compact_bits(index >> 1); // Extract every 3rd bit starting from bit 1
    let z = compact_bits(index >> 2); // Extract every 3rd bit starting from bit 2

    IVec3 { x: x as i32, y: y as i32, z: z as i32 }
}

pub fn to_morton_index_lut(x: usize, y: usize, z: usize) -> usize {
    debug_assert!(!(x | y | z) & !15 == 0);
    const MORTON: [u16; 16] = [
        0x0000, 0x0001, 0x0008, 0x0009, 0x0040, 0x0041, 0x0048, 0x0049, 0x0200, 0x0201, 0x0208,
        0x0209, 0x0240, 0x0241, 0x0248, 0x0249,
    ];
    unsafe {
        (MORTON.get_unchecked(x) | (2 * MORTON.get_unchecked(y)) | (4 * MORTON.get_unchecked(z)))
            as usize
    }
}

use std::arch::x86_64::{_pdep_u64, _pext_u64};

#[target_feature(enable = "bmi2")]
pub unsafe fn into_morton_index_bmi2(point: IVec3) -> usize {
    let x_expanded = _pdep_u64(point.x as u64, 0x2492492492492492u64); // Every 3rd bit starting at 0
    let y_expanded = _pdep_u64(point.y as u64, 0x4924924924924924u64); // Every 3rd bit starting at 1  
    let z_expanded = _pdep_u64(point.z as u64, 0x9249249249249249u64); // Every 3rd bit starting at 2
    (x_expanded | y_expanded | z_expanded) as usize
}

#[target_feature(enable = "bmi2")]
pub unsafe fn from_morton_index_bmi2(morton: usize) -> IVec3 {
    let x = _pext_u64(morton as u64, 0x2492492492492492u64); // Extract every 3rd bit starting at 0
    let y = _pext_u64(morton as u64, 0x4924924924924924u64); // Extract every 3rd bit starting at 1
    let z = _pext_u64(morton as u64, 0x9249249249249249u64); // Extract every 3rd bit starting at 2
    ivec3(x as i32, y as i32, z as i32)
}
