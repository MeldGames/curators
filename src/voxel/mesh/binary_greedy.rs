use std::collections::VecDeque;

use avian3d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::pbr::{NotShadowCaster, NotShadowReceiver};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, MeshAabb, VertexAttributeValues};
use bevy::render::render_resource::PrimitiveTopology;
use bgm::Face;
use binary_greedy_meshing::{self as bgm, Quad};

use super::UpdateVoxelMeshSet;
use crate::voxel::mesh::{ChangedChunks, chunk::VoxelChunk};
use crate::voxel::{Voxel, Voxels};

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
    commands.entity(trigger.target()).insert_if_new((
        Visibility::Inherited,
        Chunks::default(),
        VoxelsCollider(None),
    ));
}

#[derive(Component, Debug, Default, Deref, DerefMut)]
pub struct Chunks(HashMap<IVec3, Entity>);

#[derive(Component, Debug, Default, Deref, DerefMut)]
pub struct VoxelsCollider(pub Option<Entity>);

#[derive(Component, Debug, Default, Deref, DerefMut)]
pub struct ChunkCollider(pub Option<Entity>);

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
        for (chunk_pos, _) in voxels.render_chunks.chunk_iter() {
            if !voxel_chunks.contains_key(&chunk_pos) {
                let new_chunk = commands
                    .spawn((
                        Name::new(format!("Chunk [{:?}]", chunk_pos)),
                        ChunkMeshes::default(),
                        ChunkCollider::default(),
                        ChildOf(voxels_entity),
                        Transform {
                            translation: chunk_pos.as_vec3()
                                * crate::voxel::mesh::unpadded::SIZE as f32,
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
    mut grids: Query<(&Voxels, &mut Chunks), Changed<Voxels>>,
    mut chunk_mesh_entities: Query<(&mut ChunkMeshes, &mut ChunkCollider)>,

    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // voxel_materials: Res<VoxelMaterials>, // buggy when reusing material rn, figure it out later
    mut mesher: Local<BgmMesher>,
    mut changed_chunks: EventReader<ChangedChunks>,

    mut queue: Local<VecDeque<(Entity, IVec3)>>,
    mut dedup: Local<HashSet<(Entity, IVec3)>>,
) {
    for ChangedChunks { voxel_entity, changed_chunks } in changed_chunks.read() {
        for chunk in changed_chunks {
            let new_entry = (*voxel_entity, *chunk);
            if !dedup.contains(&new_entry) {
                queue.push_back(new_entry);
                dedup.insert(new_entry);
            }
        }
    }

    const PER_FRAME: usize = 16;
    let mut pop_count = 0;

    while pop_count < PER_FRAME {
        pop_count += 1;
        let Some((voxel_entity, chunk_point)) = queue.pop_front() else {
            break;
        };
        dedup.remove(&(voxel_entity, chunk_point));

        let Ok((voxels, voxel_chunks)) = grids.get_mut(voxel_entity) else {
            warn!("No voxels for entity {voxel_entity:?}");
            continue;
        };
        // collider_mesh_buffer.clear();

        let Some(chunk) = voxels.render_chunks.get_chunk(chunk_point) else {
            warn!("No chunk at {chunk_point:?}");
            continue;
        };

        // info!("chunk {:?} changed, updating binary mesh", chunk_pos);
        let render_meshes = chunk.generate_render_meshes(&mut mesher.0);
        let collider_mesh = chunk.generate_collider_mesh(&mut mesher.0);
        // collider_mesh_buffer.combine(&collider_mesh);

        let Some(chunk_entity) = voxel_chunks.get(&chunk_point) else {
            continue;
        };

        let Ok((mut chunk_meshes, mut chunk_collider)) = chunk_mesh_entities.get_mut(*chunk_entity)
        else {
            continue;
        };

        // 몰리

        for (voxel_id, render_mesh) in render_meshes.into_iter().enumerate() {
            let voxel = Voxel::from_data(voxel_id as u16);

            if let Some(entity) = chunk_meshes.get(&voxel) {
                let mut entity_commands = commands.entity(*entity);
                match render_mesh {
                    Some(mesh) => {
                        let aabb = mesh.compute_aabb();
                        let mesh_handle = meshes.add(mesh);
                        entity_commands.insert(Mesh3d(mesh_handle));
                        if let Some(aabb) = aabb {
                            entity_commands.insert(aabb);
                        }
                    },
                    None => {
                        // info!("removing mesh: {:?} {:?}", chunk_point, voxel);
                        entity_commands.remove::<Mesh3d>();
                    },
                }
            } else {
                if let Some(mesh) = render_mesh {
                    let mesh_handle = meshes.add(mesh);
                    let material = materials.add(voxel.material());
                    let mut voxel_mesh_commands = commands.spawn((
                        Name::new(format!("Voxel Mesh ({:?})", voxel.as_name())),
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(material),
                        ChildOf(*chunk_entity),
                    ));

                    if !voxel.shadow_caster() {
                        voxel_mesh_commands.insert(NotShadowCaster);
                    }
                    if !voxel.shadow_receiver() {
                        voxel_mesh_commands.insert(NotShadowReceiver);
                    }

                    let id = voxel_mesh_commands.id();

                    chunk_meshes.insert(voxel, id);
                }
            }
        }

        let collider_mesh = collider_mesh.to_mesh();

        let flags = TrimeshFlags::FIX_INTERNAL_EDGES
            | TrimeshFlags::DELETE_DEGENERATE_TRIANGLES
            | TrimeshFlags::DELETE_DUPLICATE_TRIANGLES;

        if collider_mesh.count_vertices() == 0 {
            // warn!("no vertices in collider mesh");
            continue;
        }

        let Some(mut new_collider) = Collider::trimesh_from_mesh_with_config(&collider_mesh, flags)
        else {
            info!("cannot create trimesh from mesh");
            continue;
        };
        new_collider.set_scale(crate::voxel::GRID_SCALE, 32);

        if let Some(entity) = chunk_collider.0.clone() {
            commands.entity(entity).insert(new_collider);
        } else {
            chunk_collider.0 = Some(
                commands
                    .spawn((
                        Name::new("Voxel Collider"),
                        new_collider,
                        RigidBody::Static,
                        CollisionMargin(0.05),
                        Transform::from_translation(Vec3::splat(0.0)),
                        ChildOf(*chunk_entity),
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
    fn generate_render_meshes(&self, mesher: &mut bgm::Mesher) -> Vec<Option<Mesh>>;
    fn generate_collider_mesh(&self, mesher: &mut bgm::Mesher) -> ColliderMesh;
}

pub fn pos_uvs(quad: Quad, face: Face) -> [([f32; 3], [f32; 2]); 4] {
    // UV coordinates (0..64, 0..64)
    let w = quad.width() as f32;
    let h = quad.height() as f32;
    let [x, y, z] = quad.xyz().map(|i| i as f32);
    // let w = ((MASK_6 & (quad >> 18)) as u32) as f32;
    // let h = ((MASK_6 & (quad >> 24)) as u32) as f32;
    // let xyz = (MASK_XYZ & quad) as u32;
    // let x = (MASK_6 as u32 & xyz) as f32;
    // let y = (MASK_6 as u32 & (xyz >> 6)) as f32;
    // let z = (MASK_6 as u32 & (xyz >> 12)) as f32;

    trait ArrAdd {
        fn add(self, other: Self) -> Self;
    }

    impl ArrAdd for [f32; 3] {
        fn add(self, other: Self) -> Self {
            [self[0] + other[0], self[1] + other[1], self[2] + other[2]]
        }
    }

    let pos_uvs: [([f32; 3], [f32; 2]); 4] = match face {
        Face::Left => [
            ([x, y, z], [h, w]),
            ([x, y, z].add([0., 0., h]), [0., w]),
            ([x, y, z].add([0., w, 0.]), [h, 0.]),
            ([x, y, z].add([0., w, h]), [0., 0.]),
        ],
        Face::Down => [
            ([x, y, z].add([-w, 0., h]), [w, h]),
            ([x, y, z].add([-w, 0., 0.]), [w, 0.]),
            ([x, y, z].add([0., 0., h]), [0., h]),
            ([x, y, z], [0., 0.]),
        ],
        Face::Back => [
            ([x, y, z], [w, h]),
            ([x, y, z].add([0., h, 0.]), [w, 0.]),
            ([x, y, z].add([w, 0., 0.]), [0., h]),
            ([x, y, z].add([w, h, 0.]), [0., 0.]),
        ],
        Face::Right => [
            ([x, y, z], [0., 0.]),
            ([x, y, z].add([0., 0., h]), [h, 0.]),
            ([x, y, z].add([0., -w, 0.]), [0., w]),
            ([x, y, z].add([0., -w, h]), [h, w]),
        ],
        Face::Up => [
            ([x, y, z].add([w, 0., h]), [w, h]),
            ([x, y, z].add([w, 0., 0.]), [w, 0.]),
            ([x, y, z].add([0., 0., h]), [0., h]),
            ([x, y, z], [0., 0.]),
        ],
        Face::Front => [
            ([x, y, z].add([-w, h, 0.]), [0., 0.]),
            ([x, y, z].add([-w, 0., 0.]), [0., h]),
            ([x, y, z].add([0., h, 0.]), [w, 0.]),
            ([x, y, z], [w, h]),
        ],
    };

    pos_uvs
}

impl BinaryGreedyMeshing for VoxelChunk {
    fn generate_render_meshes(&self, mesher: &mut bgm::Mesher) -> Vec<Option<Mesh>> {
        mesher.clear();
        mesher.fast_mesh_no_merge(
            &self.voxels.iter().map(|&v| v & 0xFF).collect::<Vec<_>>(),
            &self.opaque_mask,
            &self.transparent_mask,
        );

        let max_id = Voxel::iter()
            .max_by(|v1, v2| v1.id().cmp(&v2.id()))
            .map(|v| v.id() as usize)
            .expect("Some voxel to exist");

        let mut positions = vec![Vec::new(); max_id + 1];
        let mut normals = vec![Vec::new(); max_id + 1];
        let mut uvs = vec![Vec::new(); max_id + 1];
        for (face_n, quads) in mesher.quads.iter().enumerate() {
            let face: Face = (face_n as u8).into();
            let n = face.n();
            for quad in quads {
                let voxel_i = Voxel::from_data(quad.voxel_id() as u16).id() as usize;
                for (pos, uv) in pos_uvs(*quad, face) {
                    positions[voxel_i].push(pos);
                    normals[voxel_i].push(n.clone());
                    uvs[voxel_i].push(uv);
                }
            }
        }

        let mut meshes = vec![None; max_id + 1];
        for voxel in Voxel::iter() {
            let i = voxel.id() as usize;
            if voxel.rendered() && positions[i].len() > 0 {
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
                    VertexAttributeValues::Float32x2(uvs[i].clone()),
                );
                mesh.insert_indices(Indices::U32(bgm::indices(positions[i].len() / 4)));

                meshes[i] = Some(mesh);
            }
        }

        meshes
    }

    fn generate_collider_mesh(&self, mesher: &mut bgm::Mesher) -> ColliderMesh {
        let mut collide_voxels = vec![0u16; bgm::CS_P3].into_boxed_slice();
        for (index, voxel) in self.voxels.iter().enumerate() {
            if Voxel::from_data(*voxel as u16).collidable() {
                collide_voxels[index] = 1;
            }
        }

        let mut collider_mesh = ColliderMesh::default();
        mesher.clear();
        mesher.fast_mesh(&*collide_voxels, &self.opaque_mask, &self.transparent_mask);

        for (face_n, quads) in mesher.quads.iter().enumerate() {
            let face: Face = (face_n as u8).into();
            let n = face.n();
            for quad in quads {
                for (pos, _uv) in pos_uvs(*quad, face) {
                    collider_mesh.positions.push(pos);
                    collider_mesh.normals.push(n.clone());
                }
            }
        }

        collider_mesh
    }
}
