//! Voxel level editor tools

pub mod tool;

pub struct Selection {
    pub min: IVec3,
    pub max: IVec3,
}
