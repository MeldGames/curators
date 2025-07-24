//! Voxel AABB, inclusive bounds for the max

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxelAabb {
    pub min: IVec3,
    pub max: IVec3, // inclusive because voxels/integer lattice
}

impl Default for VoxelAabb {
    fn default() -> Self {
        Self { min: IVec3::ZERO, max: IVec3::ZERO }
    }
}

impl VoxelAabb {
    /// Directly construct VoxelAabb
    pub fn new(min: IVec3, max: IVec3) -> Self {
        Self { min, max }
    }

    /// Constructs an AABB from min and size
    pub fn from_size(min: IVec3, size: IVec3) -> Self {
        Self { min, max: min + size - IVec3::ONE }
    }

    pub fn as_vec3(self) -> crate::map::Aabb {
        crate::map::Aabb {
            min: self.min.as_vec3() * crate::voxel::GRID_SCALE,
            max: (self.max + IVec3::ONE).as_vec3() * crate::voxel::GRID_SCALE,
        }
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
    pub fn overlaps(&self, other: &VoxelAabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Returns the overlapping region as an AABB, or None if they don't overlap
    pub fn intersection(&self, other: &VoxelAabb) -> Option<VoxelAabb> {
        if !self.overlaps(other) {
            return None;
        }

        Some(VoxelAabb { min: self.min.max(other.min), max: self.max.min(other.max) })
    }

    /// Does this AABB fit inside another?
    pub fn fits_inside(&self, container: &VoxelAabb) -> bool {
        self.min.cmpge(container.min).all() && self.max.cmple(container.max).all()
    }

    // #[inline]
    // pub fn intersecting_chunks(&self) -> impl Iterator<Item = IVec3> {
    //     use crate::voxel::chunk::unpadded;
    //     let min = self.min.div_euclid(IVec3::splat(unpadded::SIZE_SCALAR));
    //     let max = self.max.div_euclid(IVec3::splat(unpadded::SIZE_SCALAR));

    //     (min.y..max.y).flat_map(move |y| {
    //         (min.x..max.x).flat_map(move |x| (min.z..max.z).map(move |z| IVec3::new(x, y, z)))
    //     })
    // }

    /// Returns a list of AABBs that represent self minus the overlapping region
    /// with `other`
    pub fn subtract(&self, other: &VoxelAabb) -> Vec<VoxelAabb> {
        let Some(intersection) = self.intersection(other) else {
            return vec![*self]; // no overlap, return original
        };

        let mut result = Vec::new();

        // Shortcut access
        let min = self.min;
        let max = self.max;
        let i_min = intersection.min;
        let i_max = intersection.max;

        // Split below (z-)
        if min.z < i_min.z {
            result.push(VoxelAabb {
                min: IVec3::new(min.x, min.y, min.z),
                max: IVec3::new(max.x, max.y, i_min.z - 1),
            });
        }

        // Split above (z+)
        if i_max.z < max.z {
            result.push(VoxelAabb {
                min: IVec3::new(min.x, min.y, i_max.z + 1),
                max: IVec3::new(max.x, max.y, max.z),
            });
        }

        // Remaining middle Z-slab
        let z0 = i_min.z.max(min.z);
        let z1 = i_max.z.min(max.z);

        // Split front (y-)
        if min.y < i_min.y {
            result.push(VoxelAabb {
                min: IVec3::new(min.x, min.y, z0),
                max: IVec3::new(max.x, i_min.y - 1, z1),
            });
        }

        // Split back (y+)
        if i_max.y < max.y {
            result.push(VoxelAabb {
                min: IVec3::new(min.x, i_max.y + 1, z0),
                max: IVec3::new(max.x, max.y, z1),
            });
        }

        // Remaining middle YZ-slab
        let y0 = i_min.y.max(min.y);
        let y1 = i_max.y.min(max.y);

        // Split left (x-)
        if min.x < i_min.x {
            result.push(VoxelAabb {
                min: IVec3::new(min.x, y0, z0),
                max: IVec3::new(i_min.x - 1, y1, z1),
            });
        }

        // Split right (x+)
        if i_max.x < max.x {
            result.push(VoxelAabb {
                min: IVec3::new(i_max.x + 1, y0, z0),
                max: IVec3::new(max.x, y1, z1),
            });
        }

        result
    }

    // Do Y merges aggressively because we squish vertically, most objects will be
    // taller than wider.
    pub fn can_merge_y(&self, other: &VoxelAabb) -> bool {
        self.min.x == other.min.x
            && self.max.x == other.max.x
            && self.min.z == other.min.z
            && self.max.z == other.max.z
            && (self.max.y + 1 == other.min.y || other.max.y + 1 == self.min.y)
    }

    /// Returns true if self and other are mergeable (contiguous on 1 axis, same
    /// on others)
    pub fn can_merge(&self, other: &VoxelAabb) -> bool {
        // Check axis-aligned identity for two axes, and contiguous on the third
        let same_x = self.min.x == other.min.x && self.max.x == other.max.x;
        let same_y = self.min.y == other.min.y && self.max.y == other.max.y;
        let same_z = self.min.z == other.min.z && self.max.z == other.max.z;

        let x_adjacent =
            same_y && same_z && (self.max.x + 1 == other.min.x || other.max.x + 1 == self.min.x);
        let y_adjacent =
            same_x && same_z && (self.max.y + 1 == other.min.y || other.max.y + 1 == self.min.y);
        let z_adjacent =
            same_x && same_y && (self.max.z + 1 == other.min.z || other.max.z + 1 == self.min.z);

        x_adjacent || y_adjacent || z_adjacent
    }

    /// Returns a merged AABB
    pub fn merge(&self, other: &VoxelAabb) -> VoxelAabb {
        assert!(self.can_merge(other));
        VoxelAabb { min: self.min.min(other.min), max: self.max.max(other.max) }
    }

    pub fn merge_adjacent_y_priority(mut list: Vec<VoxelAabb>) -> Vec<VoxelAabb> {
        let mut merged = true;

        while merged {
            merged = false;
            let mut new_list = Vec::with_capacity(list.len());

            while let Some(a) = list.pop() {
                if let Some((i, b)) = list
                    .iter()
                    .enumerate()
                    .find_map(|(i, b)| if a.can_merge_y(b) { Some((i, *b)) } else { None })
                {
                    list.swap_remove(i);
                    list.push(a.merge(&b));
                    merged = true;
                    break;
                } else {
                    new_list.push(a);
                }
            }

            list = new_list;
        }

        list
    }

    pub fn merge_adjacent(mut list: Vec<VoxelAabb>) -> Vec<VoxelAabb> {
        let mut changed = true;

        while changed {
            changed = false;
            let mut new_list = Vec::with_capacity(list.len());

            while let Some(a) = list.pop() {
                if let Some((i, b)) = list
                    .iter()
                    .enumerate()
                    .find_map(|(i, b)| if a.can_merge(b) { Some((i, *b)) } else { None })
                {
                    // Remove b and merge
                    list.swap_remove(i);
                    list.push(a.merge(&b));
                    changed = true;
                    break; // start over
                } else {
                    new_list.push(a);
                }
            }

            list = new_list;
        }

        list
    }

    pub fn remove_overlaps(mut list: Vec<VoxelAabb>) -> Vec<VoxelAabb> {
        let mut result: Vec<VoxelAabb> = Vec::new();

        // Remove overlaps among the list
        for current in list.drain(..) {
            let mut fragments = vec![current];

            // Remove overlaps with all boxes already in result
            for existing in &result {
                fragments =
                    fragments.into_iter().flat_map(|frag| frag.subtract(existing)).collect();
            }

            // Add non-overlapping fragments to result
            result.extend(fragments);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume() {
        let a = VoxelAabb::from_size(IVec3::new(0, 0, 0), IVec3::new(3, 3, 3));
        assert_eq!(a.volume(), 27);

        let b = VoxelAabb::from_size(IVec3::new(1, 1, 1), IVec3::new(1, 1, 1));
        assert_eq!(b.volume(), 1);

        let c = VoxelAabb::from_size(IVec3::new(2, 2, 2), IVec3::new(0, 0, 0));
        assert_eq!(c.volume(), 0); // zero-size box
    }

    #[test]
    fn overlap_true_simple() {
        let a = VoxelAabb::from_size(IVec3::new(0, 0, 0), IVec3::new(3, 3, 3));
        let b = VoxelAabb::from_size(IVec3::new(2, 2, 2), IVec3::new(3, 3, 3));

        assert!(a.overlaps(&b));
    }

    #[test]
    fn overlap_true_on_edge() {
        let a = VoxelAabb::from_size(IVec3::new(0, 0, 0), IVec3::new(2, 2, 2));
        let b = VoxelAabb::from_size(IVec3::new(1, 1, 1), IVec3::new(2, 2, 2));

        assert!(a.overlaps(&b)); // touching at corner
    }

    #[test]
    fn overlap_false_apart() {
        let a = VoxelAabb::from_size(IVec3::new(0, 0, 0), IVec3::new(2, 2, 2));
        let b = VoxelAabb::from_size(IVec3::new(5, 5, 5), IVec3::new(1, 1, 1));

        assert!(!a.overlaps(&b));
    }

    #[test]
    fn intersection_some() {
        let a = VoxelAabb::from_size(IVec3::new(0, 0, 0), IVec3::new(3, 3, 3));
        let b = VoxelAabb::from_size(IVec3::new(2, 2, 2), IVec3::new(3, 3, 3));

        let expected = VoxelAabb { min: IVec3::new(2, 2, 2), max: IVec3::new(2, 2, 2) };

        assert_eq!(a.intersection(&b), Some(expected));
    }

    #[test]
    fn intersection_none() {
        let a = VoxelAabb::from_size(IVec3::new(0, 0, 0), IVec3::new(2, 2, 2));
        let b = VoxelAabb::from_size(IVec3::new(5, 5, 5), IVec3::new(1, 1, 1));

        assert_eq!(a.intersection(&b), None);
    }

    #[test]
    fn full_containment() {
        let outer = VoxelAabb::from_size(IVec3::new(0, 0, 0), IVec3::new(5, 5, 5));
        let inner = VoxelAabb::from_size(IVec3::new(1, 1, 1), IVec3::new(3, 3, 3));

        assert!(outer.overlaps(&inner));
        assert_eq!(outer.intersection(&inner), Some(inner));
    }
}
