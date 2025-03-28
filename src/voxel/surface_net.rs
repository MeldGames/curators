use bevy::asset::RenderAssetUsages;
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, VertexAttributeValues};
use bevy::render::render_resource::{PrimitiveTopology, WgpuFeatures};
use bevy::render::settings::WgpuSettings;
use fast_surface_nets::glam::{Vec2, Vec3A};
use fast_surface_nets::ndshape::{ConstShape, ConstShape3u32, RuntimeShape, Shape};
use fast_surface_nets::{SurfaceNetsBuffer, surface_nets};

use super::voxel_grid::{Voxel, VoxelGrid};

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

pub fn update_surface_net_mesh(
    mut commands: Commands,
    mut surface_nets: Query<(Entity, &VoxelGrid, &mut SurfaceNet), Changed<VoxelGrid>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, grid, mut net) in &mut surface_nets {
        info!("!!! Updating surface net mesh !!!");
        let mut material = StandardMaterial::from(Color::srgb(0.4, 0.4, 0.4));
        material.perceptual_roughness = 0.6;

        grid.update_surface_net(&mut net.buffer);

        let mut mesh = surface_net_to_mesh(&net.buffer);
        mesh.duplicate_vertices();
        mesh.compute_flat_normals();

        commands
            .entity(entity)
            .insert((Mesh3d(meshes.add(mesh)), MeshMaterial3d(materials.add(material))));
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

impl VoxelGrid {
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
                Voxel::Dirt => -1.0,
                Voxel::Stone => -1.0,
                Voxel::Water => -1.0,
            };

            let padded_point =
                [(point[0] + 1) as u32, (point[1] + 1) as u32, (point[2] + 1) as u32];
            let padded_linear = shape.linearize(padded_point);
            samples[padded_linear as usize] = sample;
        }
        //info!("SIZES {:?} < {:?}", shape.linearize(padded_grid_array), shape.usize());

        surface_nets(
            &samples,
            &shape,
            [0; 3],
            [(grid_array[0] + 2) as u32, (grid_array[1] + 2) as u32, (grid_array[2] + 2) as u32],
            buffer,
        );
    }
}

fn spawn_pbr(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    mesh: Handle<Mesh>,
    transform: Transform,
) {
    let mut material = StandardMaterial::from(Color::srgb(0.0, 0.0, 0.0));
    material.perceptual_roughness = 0.9;

    commands.spawn((Mesh3d(mesh), MeshMaterial3d(materials.add(material)), transform));
}

fn into_domain(array_dim: u32, [x, y, z]: [u32; 3]) -> Vec3A {
    (2.0 / array_dim as f32) * Vec3A::new(x as f32, y as f32, z as f32) - 1.0
}

fn sphere(radius: f32, p: Vec3A) -> f32 {
    p.length() - radius
}

fn cube(b: Vec3A, p: Vec3A) -> f32 {
    let q = p.abs() - b;
    q.max(Vec3A::ZERO).length() + q.max_element().min(0.0)
}

fn link(le: f32, r1: f32, r2: f32, p: Vec3A) -> f32 {
    let q = Vec3A::new(p.x, (p.y.abs() - le).max(0.0), p.z);
    Vec2::new(q.length() - r1, q.z).length() - r2
}
