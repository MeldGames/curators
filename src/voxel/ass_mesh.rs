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
    pbr::wireframe::Wireframe,
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

        commands.entity(entity).insert((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(material)),
            Wireframe,
        ));
    }
}

pub struct Vertices {
    grid: Grid,
    vertices: Vec<f32>, // 0..1 for height relative to the current position
}

impl Vertices {
    pub fn from_unpadded_grid(grid: &Grid) -> Self {
        let padded = grid.pad([1; 3]);
        let vertices = vec![1.0; padded.size() as usize];
        Self { grid: padded, vertices }
    }

    pub fn weighted_vertex(&self, p: [Scalar; 3]) -> [f32; 3] {
        if self.grid.in_bounds(p) {
            let index = self.grid.linearize(p);
            let y = self.vertices[index as usize];

            [p[0] as f32, p[1] as f32 + y - 1.0, p[2] as f32]
        } else {
            [p[0] as f32, p[1] as f32, p[2] as f32]
        }
    }

    pub fn set_vertex_heights(&mut self, voxels: &VoxelGrid) {
        for vertex in &mut self.vertices {
            *vertex = 1.0;
        }

        // Each voxel has 8 vertices.
        // We use the top 4 to determine the meshing vertices and combine
        // later.
        // For each vertex we look at the 4 surrounding voxels, for each
        // occupied vertex we increase the height weighting of it by a
        // certain amount.

        for p in self.grid.point_iter() {
            let weight = |p| {
                if voxels.in_bounds(p) {
                    if let Voxel::Air = voxels.voxel(p) { 0 } else { 1 }
                } else {
                    1
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

            let occupied = w1 + w2 + w3 + w4;
            let vertex_height = match occupied {
                4 => 1.0,
                3 => 0.0,
                2 => 0.0,
                1 => 0.0,
                _ => 1.0,
            };

            //let vertex_height = w1 + w2 + w3 + w4;
            let index = self.grid.linearize(p);
            self.vertices[index as usize] = vertex_height;
        }
    }

    pub fn xz_plane_mesh(
        &self,
        voxels: &VoxelGrid,
        positions: &mut Vec<[f32; 3]>,
        indices: &mut Vec<u32>,
    ) {
        // xz plane
        for voxel_point in voxels.point_iter() {
            match voxels.voxel(voxel_point) {
                Voxel::Air => continue,
                _ => {},
            }

            match voxels.voxel([voxel_point[0], voxel_point[1] + 1, voxel_point[2]]) {
                Voxel::Air => {},
                _ => continue,
            }

            // get top vertices for this voxel
            let p1 = voxel_point;
            let p2 = [voxel_point[0] + 1, voxel_point[1], voxel_point[2]];
            let p3 = [voxel_point[0] + 1, voxel_point[1], voxel_point[2] + 1];
            let p4 = [voxel_point[0], voxel_point[1], voxel_point[2] + 1];

            let v1 = self.weighted_vertex(p1);
            let v2 = self.weighted_vertex(p2);
            let v3 = self.weighted_vertex(p3);
            let v4 = self.weighted_vertex(p4);

            let current = positions.len() as u32;
            positions.push(v1);
            positions.push(v2);
            positions.push(v3);
            positions.push(v4);
            indices.extend([current, current + 2, current + 1]);
            indices.extend([current, current + 3, current + 2]);
            // sideways
            //indices.extend([current, current + 3, current + 1]);
            //indices.extend([current + 1, current + 3, current + 2]);
        }
    }

    pub fn neg_zy_plane_mesh(
        &self,
        voxels: &VoxelGrid,
        positions: &mut Vec<[f32; 3]>,
        indices: &mut Vec<u32>,
    ) {
        // zy planes
        for voxel_point in voxels.point_iter() {
            match voxels.get_voxel(voxel_point) {
                Some(Voxel::Air) | None => continue,
                _ => {},
            }

            // zy plane
            let adjacent = [voxel_point[0] - 1, voxel_point[1], voxel_point[2]];

            match voxels.get_voxel(adjacent) {
                Some(Voxel::Air) | None => {},
                _ => continue,
            }

            if voxel_point[1] == 0 {
                continue;
            }

            // get side xy vertices for this voxel
            let p1 = [voxel_point[0], voxel_point[1], voxel_point[2]];
            let p2 = [voxel_point[0], voxel_point[1], voxel_point[2] + 1];
            let p3 = [voxel_point[0], voxel_point[1] - 1, voxel_point[2]];
            let p4 = [voxel_point[0], voxel_point[1] - 1, voxel_point[2] + 1];

            let v1 = self.weighted_vertex(p1);
            let v2 = self.weighted_vertex(p2);
            let v3 = self.weighted_vertex(p3);
            let v4 = self.weighted_vertex(p4);

            let current = positions.len() as u32;
            positions.push(v1);
            positions.push(v2);
            positions.push(v3);
            positions.push(v4);
            indices.extend([current, current + 2, current + 3]);
            indices.extend([current, current + 3, current + 1]);
        }
    }

    pub fn pos_zy_plane_mesh(
        &self,
        voxels: &VoxelGrid,
        positions: &mut Vec<[f32; 3]>,
        indices: &mut Vec<u32>,
    ) {
        // zy planes
        for voxel_point in voxels.point_iter() {
            match voxels.get_voxel(voxel_point) {
                Some(Voxel::Air) | None => continue,
                _ => {},
            }

            // zy plane
            let adjacent = [voxel_point[0] + 1, voxel_point[1], voxel_point[2]];

            match voxels.get_voxel(adjacent) {
                Some(Voxel::Air) | None => {},
                _ => continue,
            }

            if voxel_point[1] == 0 {
                continue;
            }

            // get side xy vertices for this voxel
            let p1 = [voxel_point[0] + 1, voxel_point[1], voxel_point[2]];
            let p2 = [voxel_point[0] + 1, voxel_point[1], voxel_point[2] + 1];
            let p3 = [voxel_point[0] + 1, voxel_point[1] - 1, voxel_point[2]];
            let p4 = [voxel_point[0] + 1, voxel_point[1] - 1, voxel_point[2] + 1];

            let v1 = self.weighted_vertex(p1);
            let v2 = self.weighted_vertex(p2);
            let v3 = self.weighted_vertex(p3);
            let v4 = self.weighted_vertex(p4);

            let current = positions.len() as u32;
            positions.push(v1);
            positions.push(v2);
            positions.push(v3);
            positions.push(v4);
            indices.extend([current, current + 3, current + 2]);
            indices.extend([current, current + 1, current + 3]);
        }
    }

    pub fn neg_xy_plane_mesh(
        &self,
        voxels: &VoxelGrid,
        positions: &mut Vec<[f32; 3]>,
        indices: &mut Vec<u32>,
    ) {
        // xy planes
        for voxel_point in voxels.point_iter() {
            match voxels.get_voxel(voxel_point) {
                Some(Voxel::Air) | None => continue,
                _ => {},
            }

            // xy plane
            let adjacent = [voxel_point[0], voxel_point[1], voxel_point[2] - 1];

            match voxels.get_voxel(adjacent) {
                Some(Voxel::Air) | None => {},
                _ => continue,
            }

            if voxel_point[1] == 0 {
                continue;
            }

            // get side xy vertices for this voxel
            let p1 = [voxel_point[0], voxel_point[1], voxel_point[2]];
            let p2 = [voxel_point[0] + 1, voxel_point[1], voxel_point[2]];
            let p3 = [voxel_point[0], voxel_point[1] - 1, voxel_point[2]];
            let p4 = [voxel_point[0] + 1, voxel_point[1] - 1, voxel_point[2]];

            let v1 = self.weighted_vertex(p1);
            let v2 = self.weighted_vertex(p2);
            let v3 = self.weighted_vertex(p3);
            let v4 = self.weighted_vertex(p4);

            let current = positions.len() as u32;
            positions.push(v1);
            positions.push(v2);
            positions.push(v3);
            positions.push(v4);
            indices.extend([current, current + 3, current + 2]);
            indices.extend([current, current + 1, current + 3]);
        }
    }

    pub fn pos_xy_plane_mesh(
        &self,
        voxels: &VoxelGrid,
        positions: &mut Vec<[f32; 3]>,
        indices: &mut Vec<u32>,
    ) {
        // xy planes
        for voxel_point in voxels.point_iter() {
            match voxels.get_voxel(voxel_point) {
                Some(Voxel::Air) | None => continue,
                _ => {},
            }

            // xy plane
            let adjacent = [voxel_point[0], voxel_point[1], voxel_point[2] + 1];

            match voxels.get_voxel(adjacent) {
                Some(Voxel::Air) | None => {},
                _ => continue,
            }

            if voxel_point[1] == 0 {
                continue;
            }

            // get side xy vertices for this voxel
            let p1 = [voxel_point[0], voxel_point[1], voxel_point[2] + 1];
            let p2 = [voxel_point[0] + 1, voxel_point[1], voxel_point[2] + 1];
            let p3 = [voxel_point[0], voxel_point[1] - 1, voxel_point[2] + 1];
            let p4 = [voxel_point[0] + 1, voxel_point[1] - 1, voxel_point[2] + 1];

            let v1 = self.weighted_vertex(p1);
            let v2 = self.weighted_vertex(p2);
            let v3 = self.weighted_vertex(p3);
            let v4 = self.weighted_vertex(p4);

            let current = positions.len() as u32;
            positions.push(v1);
            positions.push(v2);
            positions.push(v3);
            positions.push(v4);
            indices.extend([current, current + 2, current + 3]);
            indices.extend([current, current + 3, current + 1]);
        }
    }

    pub fn mesh(&self, voxels: &VoxelGrid) -> Mesh {
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        self.xz_plane_mesh(voxels, &mut positions, &mut indices);

        self.neg_zy_plane_mesh(voxels, &mut positions, &mut indices);
        self.pos_zy_plane_mesh(voxels, &mut positions, &mut indices);
        self.neg_xy_plane_mesh(voxels, &mut positions, &mut indices);
        self.pos_xy_plane_mesh(voxels, &mut positions, &mut indices);

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
}

pub fn grid_to_mesh(voxels: &VoxelGrid) -> Mesh {
    let mut vertices = Vertices::from_unpadded_grid(&voxels.grid);
    vertices.set_vertex_heights(&voxels);
    let mesh = vertices.mesh(voxels);

    mesh
}
