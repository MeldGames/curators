use bevy::prelude::*;
use mem_dbg::MemSize;

pub type Scalar = i32;

#[derive(MemSize, Debug, Clone, Reflect)]
pub struct Grid {
    array: [Scalar; 3],
    strides: [Scalar; 3],
    size: Scalar,
}

impl Grid {
    pub fn new([x, y, z]: [Scalar; 3]) -> Self {
        Self { array: [x, y, z], strides: [1, x, x * y], size: x * y * z }
    }

    /// Pad the this shape.
    pub fn pad(&self, padding: [Scalar; 3]) -> Self {
        let padded =
            [self.array[0] + padding[0], self.array[1] + padding[1], self.array[2] + padding[2]];
        Self::new(padded)
    }

    /// Convert this 3d point into the linear index of this grid.
    #[inline]
    pub fn linearize(&self, point: [Scalar; 3]) -> Scalar {
        point[0] + self.strides[1].wrapping_mul(point[1]) + self.strides[2].wrapping_mul(point[2])
    }

    /// Convert this index into this grid into a 3d point.
    #[inline]
    pub fn delinearize(&self, mut i: Scalar) -> [Scalar; 3] {
        let z = i / self.strides[2];
        i -= z * self.strides[2];
        let y = i / self.strides[1];
        let x = i % self.strides[1];
        [x, y, z]
    }

    /// Iterate over all points in this grid.
    pub fn point_iter(&self) -> impl Iterator<Item = [Scalar; 3]> {
        (0..self.size()).map(|i| self.delinearize(i))
    }

    /// Is this point within the bounds of this grid?
    #[inline]
    pub fn in_bounds(&self, point: [Scalar; 3]) -> bool {
        point[0] >= 0
            && point[1] >= 0
            && point[2] >= 0
            && point[0] < self.width()
            && point[1] < self.height()
            && point[2] < self.length()
    }

    #[inline]
    pub fn array(&self) -> [Scalar; 3] {
        self.array
    }

    #[inline]
    pub fn size(&self) -> Scalar {
        self.size
    }

    #[inline]
    pub fn width(&self) -> Scalar {
        self.array[0]
    }

    #[inline]
    pub fn height(&self) -> Scalar {
        self.array[1]
    }

    #[inline]
    pub fn length(&self) -> Scalar {
        self.array[2]
    }
}
