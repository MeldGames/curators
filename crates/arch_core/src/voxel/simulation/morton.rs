use bevy::prelude::*;

pub fn to_morton_index(point: IVec3) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("bmi2") {
            unsafe { x86_64::to_morton_index_bmi2(point) }
        } else {
            to_morton_index_shift(point)
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        to_morton_index_shift(point)
    }
}

pub fn from_morton_index(index: usize) -> IVec3 {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("bmi2") {
            unsafe { x86_64::from_morton_index_bmi2(index) }
        } else {
            from_morton_index_shift(index)
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        from_morton_index_shift(index)
    }
}

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
pub const fn to_morton_index_shift(point: IVec3) -> usize {
    // Interleave bits: z gets the highest bits, then y, then x
    spread_bits(point.x as usize)
        | (spread_bits(point.y as usize) << 1)
        | (spread_bits(point.z as usize) << 2)
}

#[inline]
pub const fn from_morton_index_shift(index: usize) -> IVec3 {
    // Extract interleaved bits for each coordinate
    let x = compact_bits(index); // Extract every 3rd bit starting from bit 0
    let y = compact_bits(index >> 1); // Extract every 3rd bit starting from bit 1
    let z = compact_bits(index >> 2); // Extract every 3rd bit starting from bit 2

    IVec3 { x: x as i32, y: y as i32, z: z as i32 }
}

pub fn to_morton_index_lut(point: IVec3) -> usize {
    debug_assert!(!(point.x | point.y | point.z) & !15 == 0);
    const MORTON: [u16; 16] = [
        0x0000, 0x0001, 0x0008, 0x0009, 0x0040, 0x0041, 0x0048, 0x0049, 0x0200, 0x0201, 0x0208,
        0x0209, 0x0240, 0x0241, 0x0248, 0x0249,
    ];
    unsafe {
        (MORTON.get_unchecked(point.x as usize)
            | (2 * MORTON.get_unchecked(point.y as usize))
            | (4 * MORTON.get_unchecked(point.z as usize))) as usize
    }
}

#[cfg(target_arch = "x86_64")]
mod x86_64 {
    use std::arch::x86_64::{_pdep_u64, _pext_u64};

    use bevy::prelude::*;

    pub fn to_morton_index_bmi2(point: IVec3) -> usize {
        assert!(point.x < 16 && point.y < 16 && point.z < 16);
        // x 0b1111 -> 0b001001001001;
        // y 0b1111 -> 0b010010010010;
        // z 0b1111 -> 0b100100100100;
        let combined: u64 = (point.x as u64) | ((point.y as u64) << 4) | ((point.z as u64) << 8);

        // Safety: all values are in the range of 0..16
        let expanded = unsafe { _pdep_u64(combined, 0b_100100100100_010010010010_001001001001) };
        ((expanded | (expanded >> 12) | (expanded >> 24)) & 0b111111111111) as usize
    }

    pub unsafe fn from_morton_index_bmi2(morton: usize) -> IVec3 {
        let x = _pext_u64(morton as u64, 0x2492492492492492u64); // Extract every 3rd bit starting at 0
        let y = _pext_u64(morton as u64, 0x4924924924924924u64); // Extract every 3rd bit starting at 1
        let z = _pext_u64(morton as u64, 0x9249249249249249u64); // Extract every 3rd bit starting at 2
        ivec3(x as i32, y as i32, z as i32)
    }

    #[cfg(test)]
    pub mod test {

        #[test]
        pub fn bmi_morton_same_as_simple() {
            for x in 0..16 {
                for y in 0..16 {
                    for z in 0..16 {
                        println!("{:?}", to_morton_index_bmi2(IVec3::new(x, y, z)));
                    }
                }
            }
        }
    }
}
