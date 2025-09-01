use std::cmp::Ordering;
use std::collections::VecDeque;

// use fast_surface_nets::ndshape::{ConstPow2Shape3u32, RuntimeShape, Shape};
// use fast_surface_nets::{SurfaceNetsBuffer, surface_nets};
use avian3d::collision::collider::TrimeshFlags;
use avian3d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::pbr::{NotShadowCaster, NotShadowReceiver};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, MeshAabb, VertexAttributeValues};
use bevy::render::primitives::Aabb;
use bevy::render::render_resource::PrimitiveTopology;

use crate::voxel::mesh::binary_greedy::Chunks;
use crate::voxel::mesh::chunk::VoxelChunk;
use crate::voxel::mesh::frustum_chunks::FrustumChunks;
use crate::voxel::mesh::remesh::Remesh;
use crate::voxel::mesh::surface_net::fast_surface_nets::VoxelAccess;
use crate::voxel::mesh::{ChangedChunk, padded};
use crate::voxel::simulation::data::ChunkPoint;
use crate::voxel::{UpdateVoxelMeshSet, Voxel, Voxels};

pub mod fast_surface_nets;

// use fast_surface_nets::ndshape::{Shape};
use fast_surface_nets::{SurfaceNetsBuffer, surface_nets};

pub struct SurfaceNetPlugin;
impl Plugin for SurfaceNetPlugin {
    fn build(&self, app: &mut App) {
        // app.add_observer(surface_net_components);
        app.add_systems(PreUpdate, update_surface_net_mesh.in_set(UpdateVoxelMeshSet::Mesh));
    }
}

// pub fn surface_net_components(
//     trigger: Trigger<OnAdd, Voxels>,
//     mut commands: Commands,
// ) {
//     println!("adding surface net components");
//     commands.entity(trigger.target()).insert((
//         SurfaceNetColliders::default(),
//     ));
// }

#[derive(Component, Default)]
pub struct SurfaceNet;

#[derive(Component, Default)]
pub struct SurfaceNetMesh;

#[derive(Component, Debug, Default, Deref, DerefMut)]
pub struct SurfaceNetColliders {
    voxel_colliders: HashMap<u16, Entity>, // voxel_id -> Entity
}

#[derive(Component, Debug, Default, Deref, DerefMut)]
pub struct SurfaceNetMeshes(HashMap<u16, Entity>);

// #[derive(Component, Default)]
// pub struct Remeshed(HashSet<ChunkPoint>);

pub fn update_surface_net_mesh(
    mut commands: Commands,
    is_surface_nets: Query<(), With<SurfaceNet>>,
    grids: Query<(Entity, &Voxels, &Chunks /* &mut Remeshed */)>,
    mut chunk_mesh_entities: Query<(&mut SurfaceNetMeshes, &mut SurfaceNetColliders)>,

    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // voxel_materials: Res<VoxelMaterials>, // buggy when reusing material rn, figure it out later
    // mut mesher: Local<BgmMesher>,
    mut surface_net_buffer: Local<SurfaceNetsBuffer>,
    mut changed_chunks: EventReader<ChangedChunk>,

    named: Query<NameOrEntity>,

    mut queue: Local<Vec<ChangedChunk>>,
    mut dedup: Local<HashSet<ChangedChunk>>,

    remesh: Res<Remesh>,
    frustum_chunks: Res<FrustumChunks>,

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

    for &changed_chunk in changed_chunks.read() {
        if !dedup.contains(&changed_chunk) {
            queue.push(changed_chunk);
            dedup.insert(changed_chunk);
        }
    }

    queue.sort_by(|changed_a, changed_b| {
        let ChangedChunk { grid_entity: entity_a, chunk_point: point_a } = changed_a;
        let ChangedChunk { grid_entity: entity_b, chunk_point: point_b } = changed_b;
        let a_frustum = frustum_chunks.get(&(*entity_a, *point_a));
        let b_frustum = frustum_chunks.get(&(*entity_b, *point_b));
        match (a_frustum, b_frustum) {
            (Some(_), None) => Ordering::Greater, // the one in the frustum should be placed last
            (None, Some(_)) => Ordering::Less,
            (None, None) => Ordering::Equal,
            (Some(&a_frustum), Some(&b_frustum)) => {
                a_frustum.partial_cmp(&b_frustum).unwrap_or(Ordering::Equal)
            },
        }
    });

    let mut pop_count = 0;
    while pop_count < remesh.surface_net {
        pop_count += 1;
        let Some(changed_chunk) = queue.pop() else {
            break;
        };
        dedup.remove(&changed_chunk);
        let ChangedChunk { grid_entity, chunk_point } = changed_chunk;

        let Ok((_, voxels, voxel_chunks)) = grids.get(grid_entity) else {
            warn!("No voxels for entity `{}`", named.get(grid_entity).unwrap());
            continue;
        };

        let Some(chunk_entity) = voxel_chunks.get(&chunk_point) else {
            warn!("no chunk entities for {:?}", chunk_point);
            continue;
        };

        if !is_surface_nets.contains(*chunk_entity) {
            warn!("doesn't have surface net");
            continue;
        }

        let Some(chunk) = voxels.sim_chunks.chunks.get(&chunk_point) else {
            warn!("No chunk at {chunk_point:?}");
            continue;
        };

        let Ok((mut chunk_meshes, mut chunk_colliders)) =
            chunk_mesh_entities.get_mut(*chunk_entity)
        else {
            warn!("no chunk_mesh/chunk_collider for {:?}", named.get(*chunk_entity).unwrap());
            continue;
        };

        for voxel in chunk.voxel_changeset {
            if !voxel.rendered() {
                continue;
            }

            let voxel_id = voxel.id();

            let lod = 1;
            // info!("remeshing {:?}-{:?}", chunk_point, voxel);
            // chunk.update_surface_net_samples(&mut samples.0, voxel.id());
            let chunk_size = IVec3::splat(16);
            let chunk_min = chunk_size * *chunk_point;
            let chunk_max = chunk_min + chunk_size;
            voxels.create_surface_net(
                &mut surface_net_buffer,
                voxel_id,
                chunk_min - IVec3::ONE,
                chunk_max,
                lod,
            );
            let SurfaceNetsBuffer { ref mut normals, ref mut positions, .. } = *surface_net_buffer;
            for (position, normal) in positions.iter_mut().zip(normals.iter_mut()) {
                *normal = (Vec3::from(*normal).normalize()).into();
                // const STRETCH: [f32; 3] = [0.0, 0.0, 0.0];
                const STRETCH: [f32; 3] = [0.5, 0.0, 0.5];
                *position = [
                    position[0] + normal[0] * STRETCH[0],
                    position[1] + normal[1] * STRETCH[1],
                    position[2] + normal[2] * STRETCH[2],
                ];
            }

            let mut mesh = surface_net_to_mesh(&surface_net_buffer);
            if voxel.collidable() {
                let collider_mesh = surface_net_to_collider_trimesh(&surface_net_buffer);
                match collider_mesh {
                    Some(new_collider) => {
                        // create/modify chunk collider entity
                        if let Some(entity) = chunk_colliders.get(&voxel_id) {
                            commands.entity(*entity).insert(new_collider);
                        } else {
                            let new_collider_entity = commands
                                .spawn((
                                    Name::new(format!("Voxel Collider ({:?})", voxel)),
                                    new_collider,
                                    RigidBody::Static,
                                    // CollisionMargin(0.05),
                                    Transform::from_translation(Vec3::splat(0.0)),
                                    ChildOf(*chunk_entity),
                                ))
                                .id();

                            chunk_colliders.insert(voxel_id, new_collider_entity);
                        }
                    },
                    None => {
                        // despawn chunk collider entity
                        if let Some(entity) = chunk_colliders.remove(&voxel_id) {
                            commands.entity(entity).despawn();
                        }
                    },
                }
            }

            mesh.duplicate_vertices();
            mesh.compute_flat_normals();

            // remeshed.0.insert(chunk_point);

            // 몰리

            if let Some(entity) = chunk_meshes.get(&voxel.id()) {
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
                let voxel_mesh_commands = commands.spawn((
                    // Name::new(format!("Voxel Mesh ({:?})", voxel.as_name())),
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(material),
                    ChildOf(*chunk_entity),
                    Transform {
                        translation: -crate::voxel::GRID_SCALE,
                        // scale: Vec3::splat(lod as f32),
                        ..default()
                    },
                ));

                // if !voxel.shadow_caster() {
                //     voxel_mesh_commands.insert(NotShadowCaster);
                // }
                // if !voxel.shadow_receiver() {
                //     voxel_mesh_commands.insert(NotShadowReceiver);
                // }

                let id = voxel_mesh_commands.id();

                chunk_meshes.insert(voxel.id(), id);
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

pub fn surface_net_to_collider_trimesh(buffer: &SurfaceNetsBuffer) -> Option<Collider> {
    let flags = TrimeshFlags::FIX_INTERNAL_EDGES | TrimeshFlags::DELETE_DEGENERATE_TRIANGLES;

    if buffer.positions.len() == 0 || buffer.indices.len() == 0 || buffer.indices.len() % 3 != 0 {
        // warn!("no vertices in collider mesh");
        return None;
    }

    let positions = buffer.positions.iter().copied().map(Vec3::from).collect::<Vec<_>>();
    let mut indices = Vec::new();
    for i in (0..buffer.indices.len()).step_by(3) {
        let tri = [buffer.indices[i], buffer.indices[i + 1], buffer.indices[i + 2]];
        indices.push(tri);
    }
    let mut new_collider = Collider::trimesh_with_config(positions, indices, flags);
    new_collider.set_scale(crate::voxel::GRID_SCALE, 32);

    Some(new_collider)
}

// pub type ChunkShape = ConstPow2Shape3u32<{ 6 as u32 }, { 6 as u32 }, { 6 as
// u32 }>; // 62^3 with 1 padding

pub struct LodStep<'a> {
    pub voxels: &'a Voxels,
    pub lod_step: IVec3,
}

impl<'a> VoxelAccess for LodStep<'a> {
    fn get_voxel(&self, point: IVec3) -> Voxel {
        self.voxels.get_voxel(point * self.lod_step)
    }
}

impl Voxels {
    // pub fn update_surface_net_samples(&self, samples: &mut Vec<f32>,
    // mesh_voxel_id: u16) {     let shape = ChunkShape {};
    //     for (i, voxel) in self.voxels.iter().enumerate() {
    //         let voxel = Voxel::from_data(*voxel);
    //         let voxel_id = voxel.id();

    //         let sample = if mesh_voxel_id == voxel_id {
    //             -1.0
    //         } else {
    //             1.0
    //         };

    //         let point = padded::delinearize(i);
    //         let shape_index = shape.linearize(point.map(|x| x as u32));
    //         samples[shape_index as usize] = sample;
    //     }
    // }

    pub fn create_surface_net(
        &self,
        buffer: &mut SurfaceNetsBuffer,
        mesh_voxel_id: u16,
        min: IVec3,
        max: IVec3,
        lod: i32,
    ) {
        if lod == 1 {
            surface_nets(self, mesh_voxel_id, min - 1, max, buffer);
        } else {
            let access = LodStep { voxels: self, lod_step: IVec3::splat(lod) };
            surface_nets(&access, mesh_voxel_id, min - 1 * lod, max, buffer);
        }
    }
}
