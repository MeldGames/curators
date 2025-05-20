use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, VertexAttributeValues};
use bevy::render::render_resource::{PrimitiveTopology};
use fast_surface_nets::ndshape::{RuntimeShape, Shape};
use fast_surface_nets::{SurfaceNetsBuffer, surface_nets};

use crate::voxel::{GRID_SCALE, Voxel, VoxelChunk};

pub struct SurfaceNetPlugin;
impl Plugin for SurfaceNetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_surface_net_mesh);
    }
}

#[derive(Component, Default)]
pub struct SurfaceNet {
    buffer: SurfaceNetsBuffer,
}

#[derive(Component, Default)]
pub struct SurfaceNetMesh;

pub fn update_surface_net_mesh(
    mut commands: Commands,
    mut surface_nets: Query<
        (Entity, &VoxelChunk, &mut SurfaceNet, Option<&Children>),
        Changed<VoxelChunk>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    net_meshes: Query<(), With<SurfaceNetMesh>>,
) {
    for (entity, grid, mut net, children) in &mut surface_nets {
        let material = MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.4, 0.4),
            // base_color_texture: Some(texture_mesh),
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            ..default()
        }));

        grid.update_surface_net(&mut net.buffer);

        let mut mesh = surface_net_to_mesh(&net.buffer);
        // mesh.duplicate_vertices();
        // mesh.compute_flat_normals();

        let mut mesh_entity = None;
        if let Some(children) = children {
            mesh_entity = children.iter().find(|child_entity| net_meshes.contains(*child_entity));
        }

        let mesh_entity = if let Some(mesh_entity) = mesh_entity {
            mesh_entity
        } else {
            let new_mesh_entity = commands
                .spawn((
                    Transform {
                        translation: -Vec3::new(0.5, 0.5, 0.5),
                        ..default()
                    },
                    SurfaceNetMesh,
                    Name::new("Surface nets mesh"),
                ))
                .id();

            commands.entity(entity).add_child(new_mesh_entity);

            new_mesh_entity
        };

        commands.entity(mesh_entity).insert((Mesh3d(meshes.add(mesh)), material));
    }
}

pub type VoxelShape = RuntimeShape<u32, 3>;

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

impl VoxelChunk {
    pub fn update_surface_net(&self, buffer: &mut SurfaceNetsBuffer) {
        let grid_array = self.array();
        let padded_grid_array =
            [(grid_array[0] + 3) as u32, (grid_array[1] + 3) as u32, (grid_array[2] + 3) as u32];

        let shape = VoxelShape::new(padded_grid_array);

        let mut samples = vec![1.0; shape.usize()];
        // unpadded
        for i in 0..self.size() {
            let point = self.delinearize(i);

            let sample = match self.voxel(point) {
                Voxel::Air => 1.0,
                _ => -1.0,
            };

            let padded_point =
                [(point[0] + 1) as u32, (point[1] + 1) as u32, (point[2] + 1) as u32];
            let padded_linear = shape.linearize(padded_point);
            samples[padded_linear as usize] = sample;
        }
        // info!("SIZES {:?} < {:?}", shape.linearize(padded_grid_array),
        // shape.usize());

        surface_nets(
            &samples,
            &shape,
            [0; 3],
            [(grid_array[0] + 2) as u32, (grid_array[1] + 2) as u32, (grid_array[2] + 2) as u32],
            buffer,
        );
    }
}
