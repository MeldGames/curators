//! Avian3D AnyCollider impl

use crate::voxel::voxel_grid::{Voxel, VoxelChunk};
use avian3d::prelude::*;
use bevy::prelude::*;

pub struct VoxelBoxColliderPlugin;
impl Plugin for VoxelBoxColliderPlugin {
    fn build(&self, app: &mut App) {}
}

impl AnyCollider for VoxelChunk {
    fn aabb(&self, position: avian3d::math::Vector, rotation: impl Into<Rotation>) -> ColliderAabb {
        let rotation = rotation.into();

        let min = Vec3::ZERO;
        let size_ivec: IVec3 = self.array().into();
        let size = size_ivec.as_vec3();

        let rotated_min = rotation * min;
        let rotated_max = rotation * size;

        let translated_min = position + rotated_min;
        let translated_max = position + rotated_max;

        ColliderAabb { min: translated_min, max: translated_max }
    }

    fn contact_manifolds(
        &self,
        other: &Self,
        position1: avian3d::math::Vector,
        rotation1: impl Into<Rotation>,
        position2: avian3d::math::Vector,
        rotation2: impl Into<Rotation>,
        prediction_distance: avian3d::math::Scalar,
    ) -> Vec<ContactManifold> {
        todo!()
    }
}
