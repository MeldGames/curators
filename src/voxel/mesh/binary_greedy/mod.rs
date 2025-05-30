use std::collections::BTreeSet;

use avian3d::collision::collider;
use avian3d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, VertexAttributeValues};
use bevy::render::render_resource::PrimitiveTopology;
use binary_greedy_meshing as bgm;

use super::UpdateVoxelMeshSet;
use crate::voxel::{GRID_SCALE, Voxel, VoxelChunk, Voxels};

const MASK_6: u32 = 0b111111;

pub(super) fn plugin(app: &mut App) {
    app.add_observer(add_buffers);
    app.add_systems(
        PostUpdate,
        (spawn_chunk_entities, update_binary_mesh).chain().in_set(UpdateVoxelMeshSet),
    );
}

#[derive(Component)]
pub struct BinaryGreedy;

pub fn add_buffers(trigger: Trigger<OnAdd, BinaryGreedy>, mut commands: Commands) {
    info!("adding binary greedy meshing buffers");
    commands.entity(trigger.target()).insert_if_new((Chunks::default(), VoxelsCollider(None)));
}

#[derive(Component, Debug, Default, Deref, DerefMut)]
pub struct Chunks(HashMap<IVec3, Entity>);

#[derive(Component, Debug, Default, Deref, DerefMut)]
pub struct VoxelsCollider(pub Option<Entity>);

#[derive(Component, Debug, Default, Deref, DerefMut)]
pub struct ChunkMeshes(HashMap<Voxel, Entity>);

pub struct BgmMesher(bgm::Mesher);
impl Default for BgmMesher {
    fn default() -> Self {
        Self(bgm::Mesher::new())
    }
}

pub fn spawn_chunk_entities(
    mut commands: Commands,
    mut grids: Query<(Entity, &Voxels, &mut Chunks), Changed<Voxels>>,
) {
    for (voxels_entity, voxels, mut voxel_chunks) in &mut grids {
        for (chunk_pos, _) in voxels.chunk_iter() {
            if !voxel_chunks.contains_key(&chunk_pos) {
                let new_chunk = commands
                    .spawn((
                        Name::new(format!("Chunk [{:?}]", chunk_pos)),
                        ChunkMeshes::default(),
                        ChildOf(voxels_entity),
                        Transform {
                            translation: chunk_pos.as_vec3()
                                * crate::voxel::chunk::unpadded::SIZE as f32,
                            ..default()
                        },
                        Visibility::Inherited,
                    ))
                    .id();

                voxel_chunks.insert(chunk_pos, new_chunk);
            }
        }
    }
}

pub fn update_binary_mesh(
    mut commands: Commands,
    mut grids: Query<(Entity, &Voxels, &mut Chunks, &mut VoxelsCollider), Changed<Voxels>>,
    mut chunk_mesh_entities: Query<&mut ChunkMeshes>,

    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,

    mut mesher: Local<BgmMesher>,
    mut collider_mesh_buffer: Local<ColliderMesh>,
) {
    for (voxels_entity, voxels, voxel_chunks, mut voxels_collider) in &mut grids {
        collider_mesh_buffer.clear();

        for (chunk_pos, chunk) in voxels.chunk_iter() {
            let (render_meshes, mut collider_mesh) = chunk.generate_meshes(&mut mesher.0);
            collider_mesh
                .translate(chunk_pos.as_vec3() * crate::voxel::chunk::unpadded::SIZE as f32);
            collider_mesh_buffer.combine(&collider_mesh);

            let Some(chunk_entity) = voxel_chunks.get(&chunk_pos) else {
                continue;
            };

            let Ok(mut chunk_meshes) = chunk_mesh_entities.get_mut(*chunk_entity) else {
                continue;
            };

            for (voxel_id, render_mesh) in render_meshes.into_iter().enumerate() {
                let voxel = Voxel::from_id(voxel_id as u16).unwrap();
                let Some(mesh) = render_mesh else {
                    continue;
                };
                let mesh = meshes.add(mesh);

                if let Some(entity) = chunk_meshes.get(&voxel) {
                    commands.entity(*entity).insert(Mesh3d(mesh));
                } else {
                    let material = materials.add(voxel.material());
                    let id = commands
                        .spawn((
                            Name::new(format!("Voxel Mesh ({:?})", voxel.as_name())),
                            Mesh3d(mesh),
                            MeshMaterial3d(material),
                            ChildOf(*chunk_entity),
                        ))
                        .id();

                    chunk_meshes.insert(voxel, id);
                }
            }
        }

        let collider_mesh = collider_mesh_buffer.to_mesh();

        let flags = TrimeshFlags::MERGE_DUPLICATE_VERTICES
            | TrimeshFlags::FIX_INTERNAL_EDGES
            | TrimeshFlags::DELETE_DEGENERATE_TRIANGLES
            | TrimeshFlags::DELETE_DUPLICATE_TRIANGLES;

        let Some(mut new_collider) = Collider::trimesh_from_mesh_with_config(&collider_mesh, flags)
        else {
            info!("cannot create trimesh from mesh");
            continue;
        };
        new_collider.set_scale(crate::voxel::GRID_SCALE, 32);

        if let Some(entity) = voxels_collider.0.clone() {
            commands.entity(entity).insert(new_collider);
        } else {
            voxels_collider.0 = Some(
                commands
                    .spawn((
                        Name::new("Voxels Collider"),
                        new_collider,
                        RigidBody::Static,
                        CollisionMargin(0.05),
                        Transform::from_translation(Vec3::splat(0.0)),
                        ChildOf(voxels_entity),
                    ))
                    .id(),
            );
        }
    }
}

#[derive(Default)]
pub struct ColliderMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
}

impl ColliderMesh {
    pub fn clear(&mut self) {
        self.positions.clear();
        self.normals.clear();
    }

    pub fn combine(&mut self, other: &ColliderMesh) {
        self.positions.extend(other.positions.iter());
        self.normals.extend(other.normals.iter());
    }

    pub fn translate(&mut self, by: Vec3) {
        for position in &mut self.positions {
            position[0] += by.x;
            position[1] += by.y;
            position[2] += by.z;
        }
    }

    pub fn to_mesh(&self) -> Mesh {
        let indices = bgm::indices(self.positions.len() / 4);
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            VertexAttributeValues::Float32x3(self.positions.clone()),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            VertexAttributeValues::Float32x3(self.normals.clone()),
        );
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }
}

/// Generate 1 mesh per block type for simplicity, in practice we would use a
/// texture array and a custom shader instead
pub trait BinaryGreedyMeshing {
    /// Generates 1 mesh per voxel type (voxel id is the index) and 1 collider
    /// mesh with all collidable voxels combined.
    fn generate_meshes(&self, mesher: &mut bgm::Mesher) -> (Vec<Option<Mesh>>, ColliderMesh);
}

impl BinaryGreedyMeshing for VoxelChunk {
    fn generate_meshes(&self, mesher: &mut bgm::Mesher) -> (Vec<Option<Mesh>>, ColliderMesh) {
        let mut collider_mesh = ColliderMesh::default();

        mesher.clear();
        mesher.fast_mesh(&self.voxels, &self.opaque_mask, &self.transparent_mask);

        let max_id = Voxel::iter()
            .max_by(|v1, v2| v1.id().cmp(&v2.id()))
            .map(|v| v.id() as usize)
            .expect("Some voxel to exist");

        let mut positions = vec![Vec::new(); max_id + 1];
        let mut normals = vec![Vec::new(); max_id + 1];
        let mut indices = vec![Vec::new(); max_id + 1];
        for (face_n, quads) in mesher.quads.iter().enumerate() {
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

        let mut meshes = vec![None; max_id + 1];
        for voxel in Voxel::iter() {
            let i = voxel.id() as usize;
            if voxel.filling() && positions[i].len() > 0 {
                if voxel.collidable() {
                    collider_mesh.positions.extend(positions[i].clone());
                    collider_mesh.normals.extend(normals[i].clone());
                }

                let mut mesh =
                    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
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

        (meshes, collider_mesh)
    }
}
