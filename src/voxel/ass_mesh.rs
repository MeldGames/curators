//! Aceeri's XZ Stable Y Smoothing Voxel Meshing Algorithm
//! ASS Meshing for short.
//!

use crate::voxel::voxel_grid::Voxel;

use super::{
    grid::{Grid, Scalar},
    voxel_grid::VoxelGrid,
};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
};

pub struct ASSMeshPlugin;
impl Plugin for ASSMeshPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_ass_mesh);
    }
}

#[derive(Component)]
pub struct ASSMesh;

pub fn update_ass_mesh(
    mut commands: Commands,
    mut surface_nets: Query<(Entity, &VoxelGrid, &ASSMesh), Changed<VoxelGrid>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, grid, net) in &mut surface_nets {
        info!("!!! Updating ass mesh !!!");
        let mut material = StandardMaterial::from(Color::srgb(0.4, 0.4, 0.4));
        material.perceptual_roughness = 0.6;

        let mut mesh = grid_to_mesh(grid);
        mesh.duplicate_vertices();
        mesh.compute_flat_normals();

        commands
            .entity(entity)
            .insert((Mesh3d(meshes.add(mesh)), MeshMaterial3d(materials.add(material))));
    }
}

pub struct Vertices {
    grid: Grid,
    vertices: Vec<f32>, // 0..1 for height relative to the current position
}

impl Vertices {
    pub fn weighted_vertex(&self, p: [Scalar; 3]) -> [f32; 3] {
        let index = self.grid.linearize(p);
        let y = self.vertices[index as usize];

        [p[0] as f32, p[1] as f32 + y - 1.0, p[2] as f32]
    }
}

pub fn grid_to_mesh(voxels: &VoxelGrid) -> Mesh {
    // Each voxel has 8 vertices.
    // We use the top 4 to determine the meshing vertices and combine
    // later.
    // For each vertex we look at the 4 surrounding voxels, for each
    // occupied vertex we increase the height weighting of it by a
    // certain amount.
    let padded = voxels.grid.pad([1; 3]);
    let vertices = vec![0.0; padded.size() as usize];
    let mut vertices = Vertices { grid: padded, vertices };

    for p in vertices.grid.point_iter() {
        const OCCUPIED: f32 = 0.3333;
        let weight = |p| {
            if voxels.in_bounds(p) {
                if let Voxel::Air = voxels.voxel(p) {
                    0.0 // Nothing is here, just air
                } else {
                    OCCUPIED // Something is here, weight it
                }
            } else {
                OCCUPIED // Out of bounds acts like something is there.
            }
        };

        // for this point, assume we are on the "top-left" vertex of this voxel,
        // we will need to pad the
        // surrounding voxels:
        // [p.x][p.y][p.z]
        // [p.x - 1][p.y][p.z]
        // [p.x - 1][p.y][p.z - 1]
        // [p.x][p.y][p.z - 1]
        let voxel1 = p;
        let voxel2 = [p[0] - 1, p[1], p[2]];
        let voxel3 = [p[0] - 1, p[1], p[2] - 1];
        let voxel4 = [p[0], p[1], p[2] - 1];

        let w1 = weight(voxel1);
        let w2 = weight(voxel2);
        let w4 = weight(voxel3);
        let w3 = weight(voxel4);

        let vertex_height = w1 + w2 + w3 + w4;
        let index = vertices.grid.linearize(p);
        vertices.vertices[index as usize] = vertex_height;
    }

    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for voxel_point in voxels.point_iter() {
        match voxels.voxel(voxel_point) {
            Voxel::Air => continue,
            _ => {},
        }

        match voxels.voxel([voxel_point[0], voxel_point[1] + 1, voxel_point[2]]) {
            Voxel::Air => {},
            _ => continue,
        }

        let as_f32 = |p: [i32; 3]| [p[0] as f32, p[1] as f32, p[2] as f32];

        // get top vertices for this voxel
        let p1 = voxel_point;
        let p2 = [voxel_point[0] + 1, voxel_point[1], voxel_point[2]];
        let p3 = [voxel_point[0] + 1, voxel_point[1], voxel_point[2] + 1];
        let p4 = [voxel_point[0], voxel_point[1], voxel_point[2] + 1];

        let p1_f32 = as_f32(p1);
        let p2_f32 = as_f32(p2);
        let p3_f32 = as_f32(p3);
        let p4_f32 = as_f32(p4);

        let v1 = vertices.weighted_vertex(p1);
        let v2 = vertices.weighted_vertex(p2);
        let v3 = vertices.weighted_vertex(p3);
        let v4 = vertices.weighted_vertex(p4);

        // xz plane
        let current = positions.len() as u32;
        positions.push(v1);
        positions.push(v2);
        positions.push(v3);
        positions.push(v4);
        indices.extend([current, current + 2, current + 1]);
        indices.extend([current, current + 3, current + 2]);

        // yz/yx plane
    }

    info!("{:?}", positions);
    info!("{:?}", indices);
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float32x3(positions.clone()),
    );
    mesh.insert_indices(Indices::U32(indices.clone()));
    //mesh.duplicate_vertices();
    //mesh.compute_flat_normals();
    /*mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        VertexAttributeValues::Float32x3(buffer.normals.clone()),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        VertexAttributeValues::Float32x2(vec![[0.0; 2]; num_vertices]),
    );*/

    mesh
}
