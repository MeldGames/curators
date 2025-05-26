use std::collections::BTreeSet;

use bevy::asset::RenderAssetUsages;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, VertexAttributeValues};
use bevy::render::render_resource::PrimitiveTopology;
use avian3d::prelude::*;
use binary_greedy_meshing as bgm;

use crate::voxel::{GRID_SCALE, Voxel, VoxelChunk};

use super::UpdateVoxelMeshSet;

const MASK_6: u32 = 0b111111;

pub(super) fn plugin(app: &mut App) {
    app.add_observer(add_buffers);
    app.add_systems(PostUpdate, update_binary_mesh.in_set(UpdateVoxelMeshSet));
}

#[derive(Component)]
pub struct BinaryGreedy;

#[derive(Component)]
pub struct BinaryBuffer {
    pub voxels: Vec<u16>, // 64^3 chunk, XZY ordering
}

impl Default for BinaryBuffer {
    fn default() -> Self {
        Self { voxels: vec![0; bgm::CS_P3] }
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
        ColliderMesh(None),
        BinaryBuffer::default(),
        BinaryMeshData::default(),
        ChunkMeshes::default(),
        ChunkCollider(None),
    ));
}

// Mesh generated to be used for collision
#[derive(Component, Debug, Default, Deref, DerefMut)]
pub struct ColliderMesh(pub Option<Mesh>);

#[derive(Component, Debug, Default, Deref, DerefMut)]
pub struct ChunkCollider(pub Option<Entity>);

#[derive(Component, Debug, Default, Deref, DerefMut)]
pub struct ChunkMeshes(HashMap<Voxel, Entity>);

pub fn update_binary_mesh(
    mut commands: Commands,
    mut grids: Query<(Entity, &VoxelChunk, &mut ChunkMeshes, &mut ChunkCollider, &mut BinaryBuffer, &mut BinaryMeshData), Changed<VoxelChunk>>,

    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (chunk_entity, chunk, mut chunk_meshes, mut chunk_collider, mut buffer, mut mesh_data) in &mut grids {
        let (render_meshes, collider_mesh) = chunk.generate_meshes(&mut buffer.voxels);

        let flags = TrimeshFlags::MERGE_DUPLICATE_VERTICES
            | TrimeshFlags::FIX_INTERNAL_EDGES
            | TrimeshFlags::DELETE_DEGENERATE_TRIANGLES
            | TrimeshFlags::DELETE_DUPLICATE_TRIANGLES
            ;

        let Some(mut new_collider) = Collider::trimesh_from_mesh_with_config(&collider_mesh, flags) else {
            info!("cannot create trimesh from mesh");
            continue;
        };
        new_collider.set_scale(crate::voxel::GRID_SCALE, 32);

        if let Some(entity) = chunk_collider.0.clone() {
            commands.entity(entity).insert(new_collider);
        } else {
            chunk_collider.0 = Some(commands.spawn((
                Name::new("Voxel Collider"),
                new_collider,
                RigidBody::Static,
                CollisionMargin(0.05),
                Transform::from_translation(Vec3::splat(0.0)),

                ChildOf(chunk_entity),
            )).id());
        }

        for (voxel_id, render_mesh) in render_meshes.into_iter().enumerate() {
            let voxel = Voxel::from_id(voxel_id as u16).unwrap();
            let Some(mesh) = render_mesh else { continue; };
            let mesh = meshes.add(mesh);

            if let Some(entity) = chunk_meshes.get(&voxel) {
                commands.entity(*entity).insert(Mesh3d(mesh));
            } else {
                let material = materials.add(voxel.material());
                let id = commands.spawn((
                    Name::new(format!("Voxel Mesh ({:?})", voxel.as_name())),
                    Mesh3d(mesh),
                    MeshMaterial3d(material),

                    ChildOf(chunk_entity),
                )).id();
                
                chunk_meshes.insert(voxel, id);
            }
        }
    }
}


/// Generate 1 mesh per block type for simplicity, in practice we would use a texture array and a custom shader instead
pub trait BinaryGreedyMeshing {
    fn as_binary_voxels(&self, buffer: &mut Vec<u16>);
    fn generate_meshes(&self, buffer: &mut Vec<u16>) -> (Vec<Option<Mesh>>, Mesh);
}

impl BinaryGreedyMeshing for VoxelChunk {
    fn as_binary_voxels(&self, buffer: &mut Vec<u16>) {
        for value in buffer.iter_mut() {
            *value = 0;
        }

        for (point, voxel) in self.voxel_iter() {
            let [x, y, z] = point;
            let voxel_id = if voxel.filling() { voxel.id() } else { 0 };

            buffer[bgm::pad_linearize(x as usize, y as usize, z as usize)] = voxel_id;
        }
    }

    fn generate_meshes(&self, buffer: &mut Vec<u16>) -> (Vec<Option<Mesh>>, Mesh) {
        self.as_binary_voxels(buffer);
        let mut mesh_data = bgm::MeshData::new();

        let transparents =
            Voxel::iter().filter(|v| v.transparent()).map(|v| v.id()).collect::<BTreeSet<_>>();
        let transparents = BTreeSet::new();
        bgm::mesh(&buffer, &mut mesh_data, transparents);

        let max_id = Voxel::iter().max_by(|v1, v2| v1.id().cmp(&v2.id())).map(|v| v.id() as usize).expect("Some voxel to exist");

        let mut positions = vec![Vec::new(); max_id + 1];
        let mut normals = vec![Vec::new(); max_id + 1];
        let mut indices = vec![Vec::new(); max_id + 1];
        for (face_n, quads) in mesh_data.quads.iter().enumerate() {
            let face: bgm::Face = (face_n as u8).into();
            let n = face.n();
            for quad in quads {
                let voxel_i = (quad >> 32) as usize;

                let vertices_packed = face.vertices_packed(*quad);
                for vertex_packed in vertices_packed.iter() {
                    let x = *vertex_packed & MASK_6;
                    let y = (*vertex_packed >> 6) & MASK_6;
                    let z = (*vertex_packed >> 12) & MASK_6;
                    positions[voxel_i].push([x as f32, y as f32, z as f32]);
                    normals[voxel_i].push(n.clone());

                }
            }
        }
        for i in 0..positions.len() {
            indices[i] = bgm::indices(positions[i].len() / 4);
        }


        let mut collider_positions = Vec::new();
        let mut collider_normals = Vec::new();


        let mut meshes = vec![None; max_id + 1];
        for voxel in Voxel::iter() {
            let i = voxel.id() as usize;
            if voxel.filling() && positions[i].len() > 0 {
                if voxel.collidable() {
                    collider_positions.extend(positions[i].clone());
                    collider_normals.extend(normals[i].clone());
                }

                let mut mesh = Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::RENDER_WORLD,
                );
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_POSITION,
                    VertexAttributeValues::Float32x3(positions[i].clone()),
                );
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_NORMAL,
                    VertexAttributeValues::Float32x3(normals[i].clone()),
                );
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_UV_0,
                    VertexAttributeValues::Float32x2(vec![[0.0; 2]; positions[i].len()]),
                );
                mesh.insert_indices(Indices::U32(indices[i].clone()));

                meshes[i] = Some(mesh);
            }
        }

        let collider_indices = bgm::indices(collider_positions.len() / 4);
        let mut collider_mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        collider_mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            VertexAttributeValues::Float32x3(collider_positions),
        );
        collider_mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            VertexAttributeValues::Float32x3(collider_normals),
        );
        collider_mesh.insert_indices(Indices::U32(collider_indices));

        (meshes, collider_mesh)
    }
}
