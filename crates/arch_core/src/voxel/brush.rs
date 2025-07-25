use bevy::prelude::*;

use crate::voxel::Voxels;

impl Voxels {
    // pub fn apply_brush<B: Brush>(&mut self, center: IVec3, b: B, voxel: Voxel) {
    //     for relative_affected in b.affected_voxels() {
    //         self.set_voxel(relative_affected, voxel);
    //     }
    // }
}

pub trait Brush {
    /// Voxels affected by this brush without any transformation.
    fn affected_voxels(&self) -> impl Iterator<Item = IVec3>;
}

#[derive(Debug, Clone)]
pub struct BakedBrush(Vec<IVec3>);

impl BakedBrush {
    pub fn new<B: Brush>(b: B) -> Self {
        Self(b.affected_voxels().collect())
    }
}

impl Brush for BakedBrush {
    fn affected_voxels(&self) -> impl Iterator<Item = IVec3> {
        self.0.iter().copied()
    }
}

// impl Brush for Sphere {
//     fn affected_voxels(&self) -> Vec<IVec3> {
//         let mut list = Vec::new();

//         // AABB in voxel space
//         let min = (Vec3::splat((-self.radius).floor()) / grid_scale).as_ivec3();
//         let max = (Vec3::splat(self.radius.ceil()) / grid_scale).as_ivec3();

//         for x in min.x..=max.x {
//             for y in min.y..=max.y {
//                 for z in min.z..=max.z {
//                     let point = IVec3::new(x, y, z);
//                     if self.contains_point(point.as_vec3() * grid_scale) {
//                         list.push(point);
//                     }
//                 }
//             }
//         }

//         list
//     }
// }
