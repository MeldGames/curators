use avian3d::collision::collider::TrimeshFlags;
use avian3d::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::camera::primitives::{Aabb, MeshAabb};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::{Indices, VertexAttributeValues};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use bevy_math::Affine3A;
use fast_surface_nets::ndshape::{ConstShape3u32, RuntimeShape, Shape};
use fast_surface_nets::{SurfaceNetsBuffer, surface_nets};
use priority_queue::PriorityQueue;

use crate::voxel::mesh::binary_greedy::Chunks;
use crate::voxel::mesh::chunk::VoxelChunk;
use crate::voxel::mesh::frustum_chunks::FrustumChunks;
use crate::voxel::mesh::remesh::{Remesh, RemeshCenter};
// use crate::voxel::mesh::surface_net::fast_surface_nets::VoxelAccess;
use crate::voxel::mesh::{ChangedChunk, padded};
use crate::voxel::simulation::data::ChunkPoint;
use crate::voxel::tree::CHUNK_WIDTH;
use crate::voxel::voxel::VoxelSet;
use crate::voxel::{GRID_SCALE, UpdateVoxelMeshSet, Voxel, Voxels};

// pub mod fast_surface_nets;

// use fast_surface_nets::{SurfaceNetsBuffer, surface_nets};

pub struct SurfaceNetPlugin;
impl Plugin for SurfaceNetPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SampleBuffers::default());

        // app.add_observer(surface_net_components);
        app.add_systems(PostUpdate, update_surface_net_mesh.in_set(UpdateVoxelMeshSet::Mesh));
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

pub const VOXEL_TYPE_COUNT: usize = 16;

#[derive(Resource, Debug, Clone)]
pub struct SampleBuffers {
    pub buffers: Vec<[f32; 18 * 18 * 18]>,
    pub voxel_set: VoxelSet,
}

impl Default for SampleBuffers {
    fn default() -> Self {
        Self {
            buffers: vec![[1.0; 18 * 18 * 18]; VOXEL_TYPE_COUNT],
            voxel_set: VoxelSet::default(),
        }
    }
}

impl SampleBuffers {
    pub fn clear(&mut self) {
        for voxel in self.voxel_set {
            self.buffers[voxel.id() as usize] = [1.0; 18 * 18 * 18];
        }

        // self.voxel_set.clear();
    }
}

pub fn update_surface_net_mesh(
    mut commands: Commands,
    is_surface_nets: Query<(), With<SurfaceNet>>,

    remesh_center: Res<RemeshCenter>,

    grids: Query<(Entity, &Voxels, &Chunks /* &mut Remeshed */)>,
    mut chunk_mesh_entities: Query<(&mut SurfaceNetMeshes, &mut SurfaceNetColliders)>,

    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // voxel_materials: Res<VoxelMaterials>, // buggy when reusing material rn, figure it out later
    // mut mesher: Local<BgmMesher>,
    mut surface_net_buffer: Local<SurfaceNetsBuffer>,
    mut sample_buffers: ResMut<SampleBuffers>,

    mut changed_chunks: MessageReader<ChangedChunk>,

    named: Query<NameOrEntity>,
    mut queue: Local<PriorityQueue<ChangedChunk, usize>>,

    remesh: Res<Remesh>,
    // frustum_chunks: Res<FrustumChunks>,
    mut apply_later: Local<Vec<(Entity, Handle<Mesh>, Option<Aabb>, usize)>>,
    mut gizmos: Gizmos,
) {
    #[cfg(feature = "trace")]
    let priority_span = info_span!("surface_net_priority").entered();

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

    // increase priority of all in the queue currently
    for (chunk, priority) in queue.iter_mut() {
        // center of chunk
        let chunk_size = Vec3::splat(CHUNK_WIDTH as f32) * GRID_SCALE;
        let chunk_radius = chunk_size.length() / 2.0;
        let chunk_min = chunk.chunk_point.as_vec3() * chunk_size;
        let chunk_max = chunk_min + chunk_size;
        let chunk_center = (chunk.chunk_point.as_vec3() * chunk_size) + (chunk_size / 2.0);
        let chunk_aabb = Aabb::from_min_max(chunk_min, chunk_max);

        // flash chunks in queue
        gizmos.cuboid(
            Transform { translation: chunk_center, scale: chunk_size, ..default() },
            Color::srgb(0.0, 0.0, 1.0),
        );

        // simplistic priority
        // if the chunk center is within the sphere around the priority position, then
        // prioritize more
        // gizmos.sphere(
        //     remesh_center.transform.translation,
        //     chunk_radius,
        //     Color::srgb(1.0, 0.0, 1.0),
        // );

        // base priority gain
        *priority += 1;

        // more if within a chunk radius
        let origin_distance = chunk_center.distance(remesh_center.transform.translation);
        if origin_distance < chunk_radius {
            *priority += 7;
        } else if origin_distance < chunk_radius * 2.0 {
            *priority += 6;
        } else if origin_distance < chunk_radius * 3.0 {
            *priority += 3;
        }

        // more if within camera frustum
        if remesh_center.frustum.intersects_obb(&chunk_aabb, &Affine3A::IDENTITY, true, true) {
            gizmos.sphere(chunk_center, 0.5, Color::srgb(1.0, 0.0, 1.0));

            // adjust by distance
            if origin_distance < chunk_radius {
                *priority += 4;
            } else if origin_distance < chunk_radius * 4.0 {
                *priority += 3;
            } else if origin_distance < chunk_radius * 8.0 {
                *priority += 2;
            } else {
                *priority += 1;
            }
        }
    }

    for &changed_chunk in changed_chunks.read() {
        // TODO: add the upper bounds here if they exist
        if changed_chunk.chunk_point.min_element() < 0 {
            continue;
        }

        let ChangedChunk { grid_entity, chunk_point } = changed_chunk;

        let Ok((_, _, voxel_chunks)) = grids.get(grid_entity) else {
            warn!("No voxels for entity `{}`", named.get(grid_entity).unwrap());
            continue;
        };

        let Some(chunk_entity) = voxel_chunks.get(&chunk_point) else {
            warn!("no chunk entities for {:?}", chunk_point);
            continue;
        };

        let Ok((chunk_meshes, _)) = chunk_mesh_entities.get(*chunk_entity) else {
            warn!("no chunk_mesh/chunk_collider for {:?}", named.get(*chunk_entity).unwrap());
            continue;
        };

        // set priority higher for chunks that have not been meshed
        let priority_amount = if chunk_meshes.0.len() == 0 { 20 } else { 1 };

        if !queue.change_priority_by(&changed_chunk, |priority| *priority += priority_amount) {
            queue.push(changed_chunk, priority_amount);
        }
    }

    #[cfg(feature = "trace")]
    priority_span.exit();

    let mut pop_count = 0;
    while pop_count < remesh.surface_net {
        let mut processed = false;
        let Some((changed_chunk, _popped_priority)) = queue.pop() else {
            break;
        };

        let ChangedChunk { grid_entity, chunk_point } = changed_chunk;

        let Ok((_, voxels, voxel_chunks)) = grids.get(grid_entity) else {
            warn!("No voxels for entity `{}`", named.get(grid_entity).unwrap());
            continue;
        };

        let Some(chunk_entity) = voxel_chunks.get(&chunk_point) else {
            warn!("no chunk entities for {:?}", chunk_point);
            continue;
        };

        #[cfg(feature = "trace")]
        let surface_net_iter_span = info_span!("surface_net_iter").entered();

        // if !is_surface_nets.contains(*chunk_entity) {
        //     warn!("doesn't have surface net");
        //     continue;
        // }

        let chunk_size = IVec3::splat(16);
        let chunk_min = *chunk_point * 16;
        let chunk_max = chunk_min + chunk_size;
        let min = chunk_min - IVec3::ONE;
        let max = chunk_max + IVec3::ONE;
        // info!("chunk_min: {chunk_min}, chunk_max: {chunk_max:?}");

        {
            #[cfg(feature = "trace")]
            let span = info_span!("sample").entered();
            VoxelSampler::sample(&voxels, &mut sample_buffers, min, max);
        }

        let Ok((mut chunk_meshes, mut chunk_colliders)) =
            chunk_mesh_entities.get_mut(*chunk_entity)
        else {
            warn!("no chunk_mesh/chunk_collider for {:?}", named.get(*chunk_entity).unwrap());
            continue;
        };

        for voxel in sample_buffers.voxel_set {
            if !voxel.rendered() {
                continue;
            }

            processed = true;

            let voxel_id = voxel.id();
            let sample_buffer = sample_buffers.buffers[voxel_id as usize];
            let shape = SurfaceNetShape {};
            // info!("creating surface net mesh");
            {
                #[cfg(feature = "trace")]
                let span = info_span!("generate").entered();
                surface_nets(&sample_buffer, &shape, [0; 3], [17; 3], &mut surface_net_buffer);
            }

            let SurfaceNetsBuffer { ref mut normals, ref mut positions, .. } = *surface_net_buffer;
            for (position, normal) in positions.iter_mut().zip(normals.iter_mut()) {
                *normal = (Vec3::from(*normal).normalize()).into();
                // const STRETCH: [f32; 3] = [0.0, 0.0, 0.0];
                // const STRETCH: [f32; 3] = [0.5, 0.0, 0.5];
                // *position = [
                //     position[0] + normal[0] * STRETCH[0],
                //     position[1] + normal[1] * STRETCH[1],
                //     position[2] + normal[2] * STRETCH[2],
                // ];
            }

            let mut mesh = surface_net_to_mesh(&surface_net_buffer);
            if voxel.collidable() {
                #[cfg(feature = "trace")]
                let span = info_span!("to_collider").entered();

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
                // info!("existing chunk mesh");
                // let mut entity_commands = commands.entity(*entity);
                let aabb = mesh.compute_aabb();
                let mesh_handle = meshes.add(mesh);

                apply_later.push((*entity, mesh_handle, aabb, 1)); // flickering if we try to add the mesh immediately
            // entity_commands.insert(Mesh3d(mesh_handle));
            // if let Some(aabb) = aabb {
            //     entity_commands.insert(aabb);
            // }
            } else {
                // info!("missing chunk mesh");
                // if let Some(mesh) = render_mesh {
                let mesh_handle = meshes.add(mesh);
                let material = materials.add(voxel.material());
                // let material = materials.add(voxel.material());
                // let material = voxel_materials.get(voxel);
                let mut voxel_mesh_commands = commands.spawn((
                    Name::new(format!("Voxel Mesh ({:?})", voxel.as_name())),
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(material),
                    ChildOf(*chunk_entity),
                    Transform {
                        translation: -(crate::voxel::GRID_SCALE * 2.5),
                        // scale: Vec3::splat(lod as f32),
                        ..default()
                    },
                ));

                if !voxel.shadow_caster() {
                    voxel_mesh_commands.insert(NotShadowCaster);
                }
                if !voxel.shadow_receiver() {
                    voxel_mesh_commands.insert(NotShadowReceiver);
                }

                let id = voxel_mesh_commands.id();

                chunk_meshes.insert(voxel.id(), id);
                // }
            }
        }

        if processed {
            pop_count += 1;
        }
    }
}

pub fn surface_net_to_mesh(buffer: &SurfaceNetsBuffer) -> Mesh {
    let num_vertices = buffer.positions.len();

    fn pseudo_random(index: usize) -> f32 {
        // Use a larger prime multiplier and better mixing
        let mut x = index as u64;

        // Multiply by large prime to spread out small indices
        x = x.wrapping_mul(0x9E3779B97F4A7C15);

        // XorShift-style mixing with better constants
        x ^= x >> 30;
        x = x.wrapping_mul(0xBF58476D1CE4E5B9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94D049BB133111EB);
        x ^= x >> 31;

        // Convert to 0..1 range using only the upper bits for better distribution
        ((x >> 11) as f64 / (1u64 << 53) as f64) as f32
    }

    let mut colors = Vec::with_capacity(buffer.positions.len());
    for (index, _) in buffer.positions.iter().enumerate() {
        let darkness = 0.85 + pseudo_random(index) * 0.15;
        colors.push([darkness, darkness, darkness, 1.0]);
    }

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
    // mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

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

pub struct VoxelSampler;

// 1 padding around 16^3 chunk
pub type SurfaceNetShape = ConstShape3u32<18, 18, 18>;

impl VoxelSampler {
    // min should be chunk minimum - 1
    // max is chunk maximum + 1
    // this adds the padding around the chunk
    pub fn sample(voxels: &Voxels, buffers: &mut SampleBuffers, min: IVec3, max: IVec3) {
        buffers.clear();
        let shape = SurfaceNetShape {};

        for z in min.z..max.z {
            for x in min.x..max.x {
                for y in min.y..max.y {
                    let voxel_point = IVec3::new(x, y, z);
                    let relative_point = voxel_point - min;

                    let voxel = voxels.get_voxel(voxel_point);
                    if !voxel.rendered() {
                        continue;
                    }

                    let buffer_index = voxel.id() as usize;
                    buffers.voxel_set.set(voxel);

                    let voxel_index = shape.linearize([
                        relative_point.x as u32,
                        relative_point.y as u32,
                        relative_point.z as u32,
                    ]);
                    let buffer = &mut buffers.buffers[buffer_index];
                    buffer[voxel_index as usize] = -1.0;
                }
            }
        }
    }
}
