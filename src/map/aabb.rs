use bevy::prelude::*;

// Infallible object generation algorithm:
//
// Take digsite AABBs, split aabbs on overlaps.
// Take existing objects in the voxel grid, split aabbs on object volumes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aabb {
    pub min: IVec3,
    pub max: IVec3, // inclusive
}

impl Aabb {
    /// Directly construct Aabb
    pub fn new(min: IVec3, max: IVec3) -> Self {
        Self { min, max }
    }

    /// Constructs an AABB from min and size
    pub fn from_size(min: IVec3, size: IVec3) -> Self {
        Self { min, max: min + size - IVec3::ONE }
    }

    /// Returns the size of the AABB
    pub fn size(&self) -> IVec3 {
        self.max - self.min + IVec3::ONE
    }

    /// Returns the number of voxels inside the AABB
    pub fn volume(&self) -> i32 {
        let size = self.size(); // size = max - min + 1
        size.x * size.y * size.z
    }

    /// Returns true if this AABB overlaps with another
    pub fn overlaps(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Returns the overlapping region as an AABB, or None if they don't overlap
    pub fn intersection(&self, other: &Aabb) -> Option<Aabb> {
        if !self.overlaps(other) {
            return None;
        }

        Some(Aabb { min: self.min.max(other.min), max: self.max.min(other.max) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume() {
        let a = Aabb::from_size(IVec3::new(0, 0, 0), IVec3::new(3, 3, 3));
        assert_eq!(a.volume(), 27);

        let b = Aabb::from_size(IVec3::new(1, 1, 1), IVec3::new(1, 1, 1));
        assert_eq!(b.volume(), 1);

        let c = Aabb::from_size(IVec3::new(2, 2, 2), IVec3::new(0, 0, 0));
        assert_eq!(c.volume(), 0); // zero-size box
    }

    #[test]
    fn overlap_true_simple() {
        let a = Aabb::from_size(IVec3::new(0, 0, 0), IVec3::new(3, 3, 3));
        let b = Aabb::from_size(IVec3::new(2, 2, 2), IVec3::new(3, 3, 3));

        assert!(a.overlaps(&b));
    }

    #[test]
    fn overlap_true_on_edge() {
        let a = Aabb::from_size(IVec3::new(0, 0, 0), IVec3::new(2, 2, 2));
        let b = Aabb::from_size(IVec3::new(1, 1, 1), IVec3::new(2, 2, 2));

        assert!(a.overlaps(&b)); // touching at corner
    }

    #[test]
    fn overlap_false_apart() {
        let a = Aabb::from_size(IVec3::new(0, 0, 0), IVec3::new(2, 2, 2));
        let b = Aabb::from_size(IVec3::new(5, 5, 5), IVec3::new(1, 1, 1));

        assert!(!a.overlaps(&b));
    }

    #[test]
    fn intersection_some() {
        let a = Aabb::from_size(IVec3::new(0, 0, 0), IVec3::new(3, 3, 3));
        let b = Aabb::from_size(IVec3::new(2, 2, 2), IVec3::new(3, 3, 3));

        let expected = Aabb { min: IVec3::new(2, 2, 2), max: IVec3::new(2, 2, 2) };

        assert_eq!(a.intersection(&b), Some(expected));
    }

    #[test]
    fn intersection_none() {
        let a = Aabb::from_size(IVec3::new(0, 0, 0), IVec3::new(2, 2, 2));
        let b = Aabb::from_size(IVec3::new(5, 5, 5), IVec3::new(1, 1, 1));

        assert_eq!(a.intersection(&b), None);
    }

    #[test]
    fn full_containment() {
        let outer = Aabb::from_size(IVec3::new(0, 0, 0), IVec3::new(5, 5, 5));
        let inner = Aabb::from_size(IVec3::new(1, 1, 1), IVec3::new(3, 3, 3));

        assert!(outer.overlaps(&inner));
        assert_eq!(outer.intersection(&inner), Some(inner));
    }
}
