use std::collections::VecDeque;

use bevy::asset::RenderAssetUsages;
use bevy::pbr::{NotShadowCaster, NotShadowReceiver};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, MeshAabb, VertexAttributeValues};
use bevy::render::render_resource::PrimitiveTopology;
use fast_surface_nets::{surface_nets, SurfaceNetsBuffer};
use fast_surface_nets::ndshape::{ConstPow2Shape3u32, RuntimeShape, Shape};

use crate::voxel::mesh::binary_greedy::{ChunkMeshes, Chunks};
use crate::voxel::mesh::ChangedChunks;
use crate::voxel::mesh::{chunk::VoxelChunk, padded};
use crate::voxel::{Voxel, Voxels};
use crate::voxel::mesh::binary_greedy::Remesh;

pub struct SurfaceNetPlugin;
impl Plugin for SurfaceNetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, update_surface_net_mesh);
    }
}

#[derive(Component, Default)]
pub struct SurfaceNet;

#[derive(Component, Default)]
pub struct SurfaceNetMesh;

// pub fn update_surface_net_mesh(
//     mut commands: Commands,
//     mut surface_nets: Query<
//         (Entity, &VoxelChunk, &mut SurfaceNet, Option<&Children>),
//         Changed<VoxelChunk>,
//     >,
//     mut meshes: ResMut<Assets<Mesh>>,
//     mut materials: ResMut<Assets<StandardMaterial>>,
//     net_meshes: Query<(), With<SurfaceNetMesh>>,
// ) {
//     for (entity, grid, mut net, children) in &mut surface_nets {
//         let material = MeshMaterial3d(materials.add(StandardMaterial {
//             base_color: Color::srgb(0.4, 0.4, 0.4),
//             // base_color_texture: Some(texture_mesh),
//             perceptual_roughness: 1.0,
//             reflectance: 0.0,
//             ..default()
//         }));

//         grid.update_surface_net(&mut net.buffer);

//         let mesh = surface_net_to_mesh(&net.buffer);
//         // mesh.duplicate_vertices();
//         // mesh.compute_flat_normals();

//         let mut mesh_entity = None;
//         if let Some(children) = children {
//             mesh_entity = children.iter().find(|child_entity| net_meshes.contains(*child_entity));
//         }

//         let mesh_entity = if let Some(mesh_entity) = mesh_entity {
//             mesh_entity
//         } else {
//             let new_mesh_entity = commands
//                 .spawn((
//                     Transform { translation: -Vec3::new(0.5, 0.5, 0.5), ..default() },
//                     SurfaceNetMesh,
//                     Name::new("Surface nets mesh"),
//                 ))
//                 .id();

//             commands.entity(entity).add_child(new_mesh_entity);

//             new_mesh_entity
//         };

//         commands.entity(mesh_entity).insert((Mesh3d(meshes.add(mesh)), material));
//     }
// }

pub fn update_surface_net_mesh(
    mut commands: Commands,
    mut grids: Query<(&Voxels, &Chunks), (Changed<Voxels>, With<SurfaceNet>)>,
    mut chunk_mesh_entities: Query<&mut ChunkMeshes>,

    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // voxel_materials: Res<VoxelMaterials>, // buggy when reusing material rn, figure it out later
    // mut mesher: Local<BgmMesher>,
    mut surface_net_buffer: Local<SurfaceNetsBuffer>,
    mut changed_chunks: EventReader<ChangedChunks>,

    mut queue: Local<VecDeque<(Entity, IVec3)>>,
    mut dedup: Local<HashSet<(Entity, IVec3)>>,

    remesh: Res<Remesh>,

    mut tick: Local<usize>,
) {
    *tick += 1;
    // if *tick % 8 != 0 {
    //     return;
    // }

    for ChangedChunks { voxel_entity, changed_chunks } in changed_chunks.read() {
        for chunk in changed_chunks {
            let new_entry = (*voxel_entity, *chunk);
            if !dedup.contains(&new_entry) {
                queue.push_back(new_entry);
                dedup.insert(new_entry);
            }
        }
    }

    let mut pop_count = 0;
    while pop_count < remesh.render_per_frame {
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

        chunk.update_surface_net(&mut surface_net_buffer);
        let mut mesh = surface_net_to_mesh(&surface_net_buffer);
        mesh.duplicate_vertices();
        mesh.compute_flat_normals();
        info!("mesh indices: {:?}", mesh.indices().unwrap().len());

        let Some(chunk_entity) = voxel_chunks.get(&chunk_point) else {
            continue;
        };

        let Ok(mut chunk_meshes) = chunk_mesh_entities.get_mut(*chunk_entity) else {
            continue;
        };

        // 몰리

        if let Some(entity) = chunk_meshes.get(&Voxel::Base) {
            let mut entity_commands = commands.entity(*entity);
            let aabb = mesh.compute_aabb();
            let mesh_handle = meshes.add(mesh);
            entity_commands.insert(Mesh3d(mesh_handle));
            if let Some(aabb) = aabb {
                entity_commands.insert(aabb);
            }
        } else {
            // if let Some(mesh) = render_mesh {
                let mesh_handle = meshes.add(mesh);
                let material = materials.add(Voxel::Base.material());
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

                chunk_meshes.insert(Voxel::Base, id);
            // }
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

pub type ChunkShape =
    ConstPow2Shape3u32<{ 6 as u32 }, { 6 as u32 }, { 6 as u32 }>; // 62^3 with 1 padding

impl VoxelChunk {
    pub fn update_surface_net(&self, buffer: &mut SurfaceNetsBuffer) {
        let mut samples = vec![1.0; padded::ARR_STRIDE];

        let shape = ChunkShape {};
        for (i, voxel) in self.voxels.iter().enumerate() {
            let sample = match self.voxel_from_index(i) {
                Voxel::Air => 1.0,
                _ => -1.0,
            };
            let point = padded::delinearize(i);
            let shape_index = shape.linearize(point.map(|x| x as u32));
            samples[shape_index as usize] = sample;
        }

        surface_nets(&samples, &shape, [0; 3], [padded::SIZE as u32 - 1; 3], buffer);
    }
}
