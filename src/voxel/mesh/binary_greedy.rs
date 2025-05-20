use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, VertexAttributeValues};
use bevy::render::render_resource::PrimitiveTopology;

use binary_greedy_meshing as bgm;

use crate::voxel::{GRID_SCALE, Voxel, VoxelChunk};

pub(super) fn plugin(app: &mut App) {
    app.add_observer(add_buffers);
    app.add_systems(PostUpdate, update_binary_mesh);
}

#[derive(Component)]
pub struct BinaryGreedy;

#[derive(Component)]
pub struct BinaryBuffer {
    pub voxels: [u16; bgm::CS_P3], // 64^3 chunk, XZY ordering
}

impl Default for BinaryBuffer {
    fn default() -> Self {
        Self {
            voxels: [0; bgm::CS_P3],
        }
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct BinaryMeshData(pub bgm::MeshData);
impl Default for BinaryMeshData {
    fn default() -> Self {
        Self(bgm::MeshData::new())
    }
}

pub fn add_buffers(trigger: Trigger<OnAdd, BinaryGreedy>, mut commands: Commands) {
    info!("adding binary greedy meshing buffers");
    commands.entity(trigger.target())
        .insert_if_new((
            BinaryBuffer::default(),
            BinaryMeshData::default(),
        ));
}

pub fn update_binary_mesh(mut grids: Query<(&VoxelChunk, &mut BinaryBuffer, &mut BinaryMeshData)>) {
    for (grid, mut buffer, mut mesh_data) in &mut grids {
        grid_to_buffer(&*grid, &mut buffer.voxels);
        bgm::mesh(&buffer.voxels, &mut **mesh_data, std::collections::BTreeSet::default());
    }
}

// Full grid to buffer copying
// TODO: Convert only changes to the grid to the buffer to save some copying.
pub fn grid_to_buffer(grid: &VoxelChunk, buffer: &mut [u16; bgm::CS_P3]) {
    // TODO: Clear buffer?

    for (point, voxel) in grid.voxel_iter() {
        let [x, y, z] = point;
        buffer[bgm::pad_linearize(x as usize, y as usize, z as usize)] = if voxel.filling() { 1 } else { 0 }
    }
}