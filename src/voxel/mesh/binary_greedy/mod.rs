use std::collections::BTreeSet;

use bevy::asset::RenderAssetUsages;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, VertexAttributeValues};
use bevy::render::render_resource::PrimitiveTopology;
use binary_greedy_meshing as bgm;

use crate::voxel::{GRID_SCALE, Voxel, VoxelChunk};

pub mod mesh_chunks;

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
        Self { voxels: [0; bgm::CS_P3] }
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
    commands.entity(trigger.target()).insert_if_new((
        BinaryBuffer::default(),
        BinaryMeshData::default(),
        ChunkMeshes::default(),
    ));
}

#[derive(Component, Debug, Default, Deref, DerefMut)]
pub struct ChunkMeshes(HashMap<bgm::Face, Entity>);

pub fn update_binary_mesh(
    mut commands: Commands,
    mut grids: Query<(&VoxelChunk, &mut ChunkMeshes, &mut BinaryBuffer, &mut BinaryMeshData)>,

    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (chunk, mut chunk_meshes, mut buffer, mut mesh_data) in &mut grids {
        let face_meshes = chunk.create_face_meshes(1);
        for (index, face_mesh) in face_meshes.into_iter().enumerate() {
            let face = bgm::Face::from(index as u8);
            let Some(face_mesh) = face_mesh else {
                continue;
            };

            let face_mesh = meshes.add(face_mesh);
            if let Some(entity) = chunk_meshes.get(&face) {
                commands.entity(*entity).insert(Mesh3d(face_mesh));
            } else {
                let material = materials.add(StandardMaterial { ..default() });
                let id = commands.spawn((Mesh3d(face_mesh), MeshMaterial3d(material))).id();
                chunk_meshes.insert(face, id);
            }
        }
    }
}
