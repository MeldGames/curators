use std::collections::VecDeque;

use bevy::asset::RenderAssetUsages;
use bevy::pbr::{NotShadowCaster, NotShadowReceiver};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, MeshAabb, VertexAttributeValues};
use bevy::render::primitives::Aabb;
use bevy::render::render_resource::PrimitiveTopology;
use fast_surface_nets::ndshape::{ConstPow2Shape3u32, RuntimeShape, Shape};
use fast_surface_nets::{SurfaceNetsBuffer, surface_nets};

use crate::voxel::mesh::ChangedChunks;
use crate::voxel::mesh::binary_greedy::Remesh;
use crate::voxel::mesh::binary_greedy::{ChunkMeshes, Chunks};
use crate::voxel::mesh::{chunk::VoxelChunk, padded};
use crate::voxel::{Voxel, Voxels};

pub struct SurfaceNetPlugin;
impl Plugin for SurfaceNetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, update_surface_net_mesh);
    }
}

pub struct SamplesBuffer(Vec<f32>);
impl Default for SamplesBuffer {
    fn default() -> Self {
        Self(vec![1.0; padded::ARR_STRIDE])
    }
}

#[derive(Component, Default)]
pub struct SurfaceNet;

#[derive(Component, Default)]
pub struct SurfaceNetMesh;

pub fn update_surface_net_mesh(
    mut commands: Commands,
    is_surface_nets: Query<(), With<SurfaceNet>>,
    mut grids: Query<(&Voxels, &Chunks), Changed<Voxels>>,
    mut chunk_mesh_entities: Query<&mut ChunkMeshes>,

    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // voxel_materials: Res<VoxelMaterials>, // buggy when reusing material rn, figure it out later
    // mut mesher: Local<BgmMesher>,
    mut surface_net_buffer: Local<SurfaceNetsBuffer>,
    mut changed_chunks: EventReader<ChangedChunks>,

    mut samples: Local<SamplesBuffer>,

    mut queue: Local<VecDeque<(Entity, IVec3)>>,
    mut dedup: Local<HashSet<(Entity, IVec3)>>,

    remesh: Res<Remesh>,

    mut apply_later: Local<Vec<(Entity, Handle<Mesh>, Option<Aabb>, usize)>>,
) {
    apply_later.retain(|(_, _, _, count)| *count != 0);

    for (entity, mesh, aabb, count) in &mut apply_later {
        // info!("count: {:?}", count);
        *count -= 1;
        if *count == 0 {
            // info!("adding mesh");
            let mut entity_commands = commands.entity(*entity);
            entity_commands.insert(Mesh3d(mesh.clone()));
            if let Some(aabb) = aabb {
                entity_commands.insert(*aabb);
            }
        }
    }

    for ChangedChunks { voxel_entity, changed_chunks } in changed_chunks.read() {
        for chunk in changed_chunks {
            let new_entry = (*voxel_entity, *chunk);
            if !dedup.contains(&new_entry) {
                queue.push_back(new_entry);
                dedup.insert(new_entry);
            }
        }
    }

    // if !input.just_pressed(KeyCode::KeyY) {
    //     return;
    // }

    let mut pop_count = 0;
    while pop_count < remesh.render_per_frame {
        pop_count += 1;
        let Some((voxel_entity, chunk_point)) = queue.pop_front() else {
            break;
        };
        dedup.remove(&(voxel_entity, chunk_point));

        if !is_surface_nets.contains(voxel_entity) {
            continue;
        }

        let Ok((voxels, voxel_chunks)) = grids.get_mut(voxel_entity) else {
            warn!("No voxels for entity {voxel_entity:?}");
            continue;
        };

        let Some(chunk) = voxels.render_chunks.get_chunk(chunk_point) else {
            warn!("No chunk at {chunk_point:?}");
            continue;
        };

        for voxel in [Voxel::Base, Voxel::Dirt, Voxel::Sand, Voxel::Water { lateral_energy: 4 }] {
            if !voxel.rendered() {
                continue;
            }

            chunk.update_surface_net_samples(&mut samples.0, voxel.id());
            chunk.create_surface_net(&mut surface_net_buffer, &mut samples.0);
            for normal in surface_net_buffer.normals.iter_mut() {
                *normal = (Vec3::from(*normal).normalize()).into();
            }

            let mut mesh = surface_net_to_mesh(&surface_net_buffer);
            mesh.duplicate_vertices();
            mesh.compute_flat_normals();

            let Some(chunk_entity) = voxel_chunks.get(&chunk_point) else {
                continue;
            };

            let Ok(mut chunk_meshes) = chunk_mesh_entities.get_mut(*chunk_entity) else {
                continue;
            };

            // 몰리

            if let Some(entity) = chunk_meshes.get(&voxel) {
                // let mut entity_commands = commands.entity(*entity);
                let aabb = mesh.compute_aabb();
                let mesh_handle = meshes.add(mesh);

                apply_later.push((*entity, mesh_handle, aabb, 1)); // flickering if we try to add the mesh immediately
            // entity_commands.insert(Mesh3d(mesh_handle));
            // if let Some(aabb) = aabb {
            //     entity_commands.insert(aabb);
            // }
            } else {
                // if let Some(mesh) = render_mesh {
                let mesh_handle = meshes.add(mesh);
                let material = materials.add(voxel.material());
                // let material = materials.add(voxel.material());
                // let material = voxel_materials.get(voxel);
                let mut voxel_mesh_commands = commands.spawn((
                    // Name::new(format!("Voxel Mesh ({:?})", voxel.as_name())),
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(material),
                    ChildOf(*chunk_entity),
                ));

                // if !voxel.shadow_caster() {
                //     voxel_mesh_commands.insert(NotShadowCaster);
                // }
                // if !voxel.shadow_receiver() {
                //     voxel_mesh_commands.insert(NotShadowReceiver);
                // }

                let id = voxel_mesh_commands.id();

                chunk_meshes.insert(voxel, id);
                // }
            }
        }
    }
}

pub fn surface_net_to_mesh(buffer: &SurfaceNetsBuffer) -> Mesh {
    let num_vertices = buffer.positions.len();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float32x3(buffer.positions.clone()),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        VertexAttributeValues::Float32x3(buffer.normals.clone()),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        VertexAttributeValues::Float32x2(vec![[0.0; 2]; num_vertices]),
    );
    mesh.insert_indices(Indices::U32(buffer.indices.clone()));

    mesh
}

pub type ChunkShape = ConstPow2Shape3u32<{ 6 as u32 }, { 6 as u32 }, { 6 as u32 }>; // 62^3 with 1 padding

impl VoxelChunk {
    pub fn update_surface_net_samples(&self, samples: &mut Vec<f32>, mesh_voxel_id: u16) {
        let shape = ChunkShape {};
        for (i, voxel) in self.voxels.iter().enumerate() {
            let voxel = Voxel::from_data(*voxel);
            let voxel_id = voxel.id();

            let sample = if mesh_voxel_id == voxel_id {
                -1.0
            } else {
                1.0
            };

            let point = padded::delinearize(i);
            let shape_index = shape.linearize(point.map(|x| x as u32));
            samples[shape_index as usize] = sample;
        }
    }

    pub fn create_surface_net(&self, buffer: &mut SurfaceNetsBuffer, samples: &Vec<f32>) {
        let shape = ChunkShape {};
        surface_nets(&samples, &shape, [0; 3], [padded::SIZE as u32 - 1; 3], buffer);
    }
}
