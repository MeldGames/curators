use bevy::prelude::*;
use mem_dbg::MemSize;

pub type Scalar = i32;

#[derive(MemSize, Debug, Clone, Reflect)]
pub struct Grid {
    array: [Scalar; 3],
    strides: [Scalar; 3],
    ordering: Ordering,
    size: Scalar,
}

#[derive(Copy, Clone, Reflect, MemSize, Debug)]
pub enum Ordering {
    XYZ,
    XZY,
    ZYX,
    ZXY,
    YXZ,
    YZX,
}

impl Ordering {
    pub fn strides(&self, [x, y, z]: [Scalar; 3]) -> [Scalar; 3] {
        match self {
            Ordering::XYZ => [1, x, x * y],
            Ordering::XZY => [1, x * z, x],
            Ordering::ZYX => [z * y, z, 1],
            Ordering::ZXY => [z, z * x, 1],
            Ordering::YXZ => [y, 1, y * x],
            Ordering::YZX => [y * x, 1, y],
        }
    }
}

impl Grid {
    pub fn new([x, y, z]: [Scalar; 3], ordering: Ordering) -> Self {
        Self { array: [x, y, z], strides: ordering.strides([x, y, z]), ordering, size: x * y * z }
    }

    /// Pad the this shape.
    pub fn pad(&self, padding: [Scalar; 3]) -> Self {
        let padded =
            [self.array[0] + padding[0], self.array[1] + padding[1], self.array[2] + padding[2]];
        Self::new(padded, self.ordering)
    }

    /// Convert this 3d point into the linear index of this grid.
    #[inline]
    pub fn linearize(&self, point: [Scalar; 3]) -> Scalar {
        self.strides[0].wrapping_mul(point[0])
            + self.strides[1].wrapping_mul(point[1])
            + self.strides[2].wrapping_mul(point[2])
    }

    /// Convert this index into this grid into a 3d point.
    #[inline]
    pub fn delinearize(&self, mut i: Scalar) -> [Scalar; 3] {
        let [s1, s2] = match self.ordering {
            Ordering::XYZ => [self.strides[1], self.strides[2]],
            Ordering::XZY => [self.strides[2], self.strides[1]],
            _ => [self.strides[1], self.strides[2]],
        };

        let n2 = i / s2;
        i -= n2 * s2;
        let n1 = i / s1;
        let n0 = i % s1;

        match self.ordering {
            Ordering::XYZ => [n0, n1, n2],
            Ordering::XZY => [n0, n2, n1],
            _ => [n0, n1, n2],
        }
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
            && point[0] < self.array[0]
            && point[1] < self.array[1]
            && point[2] < self.array[2]
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
    pub fn x(&self) -> Scalar {
        match self.ordering {
            Ordering::XYZ => self.array[0],
            Ordering::XZY => self.array[0],
            _ => todo!(),
        }
    }

    #[inline]
    pub fn y(&self) -> Scalar {
        self.array[1]
        // match self.ordering {
        // Ordering::XYZ => self.array[1],
        // Ordering::XZY => self.array[2],
        // _ => todo!(),
        // }
    }

    #[inline]
    pub fn z(&self) -> Scalar {
        self.array[2]
        // match self.ordering {
        // Ordering::XYZ => self.array[2],
        // Ordering::XZY => self.array[1],
        // _ => todo!(),
        // }
    }

    #[inline]
    pub fn scaled_bounds(&self) -> Vec3 {
        self.bounds() * crate::voxel::GRID_SCALE
    }

    #[inline]
    pub fn bounds(&self) -> Vec3 {
        Vec3::new(self.x() as f32, self.y() as f32, self.z() as f32)
    }
}
