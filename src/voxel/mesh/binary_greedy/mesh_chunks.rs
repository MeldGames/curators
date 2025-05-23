use std::collections::BTreeSet;

use bevy::log::info_span;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, MeshVertexAttribute};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{PrimitiveTopology, VertexFormat};
use binary_greedy_meshing as bgm;

use super::material::{TextureMap, TextureMapTrait};
use crate::voxel::{Voxel, VoxelChunk};

const MASK_6: u64 = 0b111111;
const MASK_XYZ: u64 = 0b111111_111111_111111;
/// ## Compressed voxel vertex data
/// first u32 (vertex dependent):
///     - chunk position: 3x6 bits (33 values)
///     - texture coords: 2x6 bits (33 values)
///     - ambient occlusion?: 2 bits (4 values)
/// `0bao_vvvvvv_uuuuuu_zzzzzz_yyyyyy_xxxxxx`
///
/// second u32 (vertex agnostic):
///     - normals: 3 bits (6 values) = face
///     - color: 9 bits (3 r, 3 g, 3 b)
///     - texture layer: 16 bits
///     - light level: 4 bits (16 value)
///
/// `0bllll_iiiiiiiiiiiiiiii_ccccccccc_nnn`
pub const ATTRIBUTE_VOXEL_DATA: MeshVertexAttribute =
    MeshVertexAttribute::new("VoxelData", 48757581, VertexFormat::Uint32x2);

impl VoxelChunk {
    pub fn as_binary_voxels(&self) -> [u16; bgm::CS_P3] {
        let mut buffer = [0u16; bgm::CS_P3];
        for (point, voxel) in self.voxel_iter() {
            let [x, y, z] = point;
            let voxel_id = if voxel.filling() { voxel.id() } else { 0 };

            buffer[bgm::pad_linearize(x as usize, y as usize, z as usize)] = voxel_id;
        }
        buffer
    }

    /// Doesn't work with lod > 2, because chunks are of size 62 (to get to 64
    /// with padding) and 62 = 2*31 TODO: make it work with lod > 2 if
    /// necessary (by truncating quads)
    pub fn create_face_meshes(
        &self,
        texture_map: impl TextureMapTrait,
        lod: usize,
    ) -> [Option<Mesh>; 6] {
        // Gathering binary greedy meshing input data
        let mesh_data_span = info_span!("mesh voxel data", name = "mesh voxel data").entered();
        let voxels = self.as_binary_voxels();
        let mut mesh_data = bgm::MeshData::new();
        mesh_data_span.exit();

        let mesh_build_span = info_span!("mesh build", name = "mesh build").entered();
        let transparents =
            Voxel::iter().filter(|v| v.transparent()).map(|v| v.id()).collect::<BTreeSet<_>>();

        bgm::mesh(&voxels, &mut mesh_data, transparents);

        info!("mark 1");
        let mut meshes = std::array::from_fn(|_| None);
        for (face_n, quads) in mesh_data.quads.iter().enumerate() {
            let mut voxel_data: Vec<[u32; 2]> = Vec::with_capacity(quads.len() * 4);
            let indices = bgm::indices(quads.len());
            let face: bgm::Face = (face_n as u8).into();

            // TODO: Split this off into a per voxel mesh?
            for quad in quads {
                let voxel_i = (quad >> 32) as usize;
                let voxel = Voxel::from_id(voxel_i as u16).expect("Voxel id to be valid");

                let layer = texture_map.get_texture_index(voxel, face) as u32;
                let color = match (voxel, face) {
                    (Voxel::Grass, bgm::Face::Up) => 0b011_111_001,
                    (Voxel::Grass, _) => 0b110_011_001,
                    _ => 0b111_111_111,
                };

                let vertices = face.vertices_packed(*quad);
                let quad_info = (layer << 12) | (color << 3) | face_n as u32;

                voxel_data.extend_from_slice(&[
                    [vertices[0], quad_info],
                    [vertices[1], quad_info],
                    [vertices[2], quad_info],
                    [vertices[3], quad_info],
                ]);
            }
            meshes[face_n] = Some(
                Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                    .with_inserted_attribute(ATTRIBUTE_VOXEL_DATA, voxel_data)
                    .with_inserted_indices(Indices::U32(indices)),
            )
        }
        mesh_build_span.exit();
        meshes
    }
}
