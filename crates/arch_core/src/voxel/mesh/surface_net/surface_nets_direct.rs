//! Taken from https://github.com/bonsairobo/fast-surface-nets
//!
//! Modified to work directly with our voxel grids instead of us converting to a
//! sample grid as well as various optimizations based on the fact we aren't
//! actually dealing with an SDF, instead we use binary data of "voxel is(n't)
//! here".
//!
//! Normal gradients are also unnecessary for us, we just want a flat normal for
//! each triangle, this is entirely stylistic.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::voxel::{Voxel, Voxels};

pub type VoxelData = u16;
pub type VoxelId = u16;

/// The output buffers used by [`surface_nets`]. These buffers can be reused to
/// avoid reallocating memory.
#[derive(Default, Clone)]
pub struct SurfaceNetsBuffer {
    /// The triangle mesh positions.
    pub positions: Vec<[f32; 3]>,
    /// The triangle mesh normals.
    pub normals: Vec<[f32; 3]>,
    /// The triangle mesh indices.
    pub indices: Vec<u32>,
    /// Local 3D array coordinates of every voxel that intersects the
    /// isosurface.
    pub surface_points: Vec<IVec3>,
    /// Used to map back from voxel position to vertex index.
    pub position_to_index: HashMap<IVec3, u32>,
}

impl SurfaceNetsBuffer {
    /// Clears all of the buffers, but keeps the memory allocated for reuse.
    fn reset(&mut self) {
        self.positions.clear();
        self.normals.clear();
        self.indices.clear();
        self.surface_points.clear();
        self.position_to_index.clear();
    }
}

/// The Naive Surface Nets smooth voxel meshing algorithm.
pub fn surface_nets(
    voxels: &Voxels,
    mesh_voxel_id: VoxelId,
    min_point: IVec3,
    max_point: IVec3,
    output: &mut SurfaceNetsBuffer,
) {
    output.reset();

    // Add padding for edge detection but don't mesh the padding
    let padded_min = min_point - IVec3::ONE;
    let padded_max = max_point + IVec3::ONE;

    estimate_surface(voxels, mesh_voxel_id, min_point, max_point, padded_min, padded_max, output);
    make_all_quads(voxels, mesh_voxel_id, min_point, max_point, output);
}

// Find all vertex positions and normals. Also generate a map from grid position
// to vertex index to be used to look up vertices when generating quads.
fn estimate_surface(
    voxels: &Voxels,
    mesh_voxel_id: VoxelId,
    min_point: IVec3,
    max_point: IVec3,
    padded_min: IVec3,
    padded_max: IVec3,
    output: &mut SurfaceNetsBuffer,
) {
    for x in min_point.x..max_point.x {
        for y in min_point.y..max_point.y {
            for z in min_point.z..max_point.z {
                let pos = IVec3::new(x, y, z);
                let p = Vec3A::from([x as f32, y as f32, z as f32]);

                if estimate_surface_in_cube(
                    voxels,
                    mesh_voxel_id,
                    pos,
                    padded_min,
                    padded_max,
                    p,
                    output,
                ) {
                    let vertex_index = output.positions.len() as u32 - 1;
                    output.position_to_index.insert(pos, vertex_index);
                    output.surface_points.push(pos);
                }
            }
        }
    }
}

// Consider the grid-aligned cube where `pos` is the minimal corner. Find a
// point inside this cube that is approximately on the isosurface.
fn estimate_surface_in_cube(
    voxels: &Voxels,
    mesh_voxel_id: VoxelId,
    pos: IVec3,
    padded_min: IVec3,
    padded_max: IVec3,
    p: Vec3A,
    output: &mut SurfaceNetsBuffer,
) -> bool {
    // Get the signed distance values at each corner of this cube.
    let mut corner_dists = [0f32; 8];
    let mut num_negative = 0;

    for (i, dist) in corner_dists.iter_mut().enumerate() {
        let corner_offset = CUBE_CORNERS[i];
        let corner_pos = IVec3::new(
            pos.x + corner_offset[0] as i32,
            pos.y + corner_offset[1] as i32,
            pos.z + corner_offset[2] as i32,
        );

        // Use padded bounds for voxel sampling
        let voxel = if corner_pos.x >= padded_min.x
            && corner_pos.x <= padded_max.x
            && corner_pos.y >= padded_min.y
            && corner_pos.y <= padded_max.y
            && corner_pos.z >= padded_min.z
            && corner_pos.z <= padded_max.z
        {
            voxels.get_voxel(corner_pos)
        } else {
            Voxel::Air
        };

        *dist = if voxel.id() == mesh_voxel_id {
            num_negative += 1;
            -1.0
        } else {
            1.0
        };
    }

    if num_negative == 0 || num_negative == 8 {
        // No crossings.
        return false;
    }

    let c = centroid_of_edge_intersections(&corner_dists);

    output.positions.push((p + c).into());
    output.normals.push(sdf_gradient(&corner_dists, c).into());

    true
}

fn centroid_of_edge_intersections(dists: &[f32; 8]) -> Vec3A {
    let mut count = 0;
    let mut sum = Vec3A::ZERO;
    for &[corner1, corner2] in CUBE_EDGES.iter() {
        let d1 = dists[corner1 as usize];
        let d2 = dists[corner2 as usize];
        if (d1 < 0.0) != (d2 < 0.0) {
            count += 1;
            sum += estimate_surface_edge_intersection(corner1, corner2, d1, d2);
        }
    }

    if count == 0 {
        Vec3A::splat(0.5) // fallback to center
    } else {
        sum / count as f32
    }
}

// Given two cube corners, find the point between them where the SDF is zero.
fn estimate_surface_edge_intersection(
    corner1: u32,
    corner2: u32,
    value1: f32,
    value2: f32,
) -> Vec3A {
    let interp1 = value1 / (value1 - value2);
    let interp2 = 1.0 - interp1;

    interp2 * CUBE_CORNER_VECTORS[corner1 as usize]
        + interp1 * CUBE_CORNER_VECTORS[corner2 as usize]
}

/// Calculate the normal as the gradient of the distance field.
fn sdf_gradient(dists: &[f32; 8], s: Vec3A) -> Vec3A {
    let p00 = Vec3A::from([dists[0b001], dists[0b010], dists[0b100]]);
    let n00 = Vec3A::from([dists[0b000], dists[0b000], dists[0b000]]);

    let p10 = Vec3A::from([dists[0b101], dists[0b011], dists[0b110]]);
    let n10 = Vec3A::from([dists[0b100], dists[0b001], dists[0b010]]);

    let p01 = Vec3A::from([dists[0b011], dists[0b110], dists[0b101]]);
    let n01 = Vec3A::from([dists[0b010], dists[0b100], dists[0b001]]);

    let p11 = Vec3A::from([dists[0b111], dists[0b111], dists[0b111]]);
    let n11 = Vec3A::from([dists[0b110], dists[0b101], dists[0b011]]);

    let d00 = p00 - n00;
    let d10 = p10 - n10;
    let d01 = p01 - n01;
    let d11 = p11 - n11;

    let neg = Vec3A::ONE - s;

    neg.yzx() * neg.zxy() * d00
        + neg.yzx() * s.zxy() * d10
        + s.yzx() * neg.zxy() * d01
        + s.yzx() * s.zxy() * d11
}

// For every edge that crosses the isosurface, make a quad between the "centers"
// of the four cubes touching that surface.
fn make_all_quads(
    voxels: &Voxels,
    mesh_voxel_id: VoxelId,
    min_point: IVec3,
    max_point: IVec3,
    output: &mut SurfaceNetsBuffer,
) {
    for &pos in &output.surface_points {
        // Do edges parallel with the X axis
        if pos.y > min_point.y && pos.z > min_point.z && pos.x < max_point.x - 1 {
            maybe_make_quad(
                voxels,
                mesh_voxel_id,
                &output.position_to_index,
                &output.positions,
                pos,
                IVec3::new(pos.x + 1, pos.y, pos.z),
                IVec3::new(0, -1, 0), // axis B
                IVec3::new(0, 0, -1), // axis C
                &mut output.indices,
            );
        }
        // Do edges parallel with the Y axis
        if pos.x > min_point.x && pos.z > min_point.z && pos.y < max_point.y - 1 {
            maybe_make_quad(
                voxels,
                mesh_voxel_id,
                &output.position_to_index,
                &output.positions,
                pos,
                IVec3::new(pos.x, pos.y + 1, pos.z),
                IVec3::new(0, 0, -1), // axis B
                IVec3::new(-1, 0, 0), // axis C
                &mut output.indices,
            );
        }
        // Do edges parallel with the Z axis
        if pos.x > min_point.x && pos.y > min_point.y && pos.z < max_point.z - 1 {
            maybe_make_quad(
                voxels,
                mesh_voxel_id,
                &output.position_to_index,
                &output.positions,
                pos,
                IVec3::new(pos.x, pos.y, pos.z + 1),
                IVec3::new(-1, 0, 0), // axis B
                IVec3::new(0, -1, 0), // axis C
                &mut output.indices,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn maybe_make_quad(
    voxels: &Voxels,
    mesh_voxel_id: VoxelId,
    position_to_index: &HashMap<IVec3, u32>,
    positions: &[[f32; 3]],
    p1: IVec3,
    p2: IVec3,
    axis_b: IVec3,
    axis_c: IVec3,
    indices: &mut Vec<u32>,
) {
    let v1_voxel = voxels.get_voxel(p1);
    let v2_voxel = voxels.get_voxel(p2);

    let negative_face = match (v1_voxel.id() == mesh_voxel_id, v2_voxel.id() == mesh_voxel_id) {
        (true, false) => false,
        (false, true) => true,
        _ => return, // No face.
    };

    // Get the four vertices of the quad
    let pos1 = p1;
    let pos2 = IVec3::new(p1.x + axis_b.x, p1.y + axis_b.y, p1.z + axis_b.z);
    let pos3 = IVec3::new(p1.x + axis_c.x, p1.y + axis_c.y, p1.z + axis_c.z);
    let pos4 = IVec3::new(
        p1.x + axis_b.x + axis_c.x,
        p1.y + axis_b.y + axis_c.y,
        p1.z + axis_b.z + axis_c.z,
    );

    // Look up vertex indices
    let v1 = position_to_index.get(&pos1).copied().unwrap_or(u32::MAX);
    let v2 = position_to_index.get(&pos2).copied().unwrap_or(u32::MAX);
    let v3 = position_to_index.get(&pos3).copied().unwrap_or(u32::MAX);
    let v4 = position_to_index.get(&pos4).copied().unwrap_or(u32::MAX);

    // Skip if any vertex is missing
    if v1 == u32::MAX || v2 == u32::MAX || v3 == u32::MAX || v4 == u32::MAX {
        return;
    }

    let (pos1_3d, pos2_3d, pos3_3d, pos4_3d) = (
        Vec3A::from(positions[v1 as usize]),
        Vec3A::from(positions[v2 as usize]),
        Vec3A::from(positions[v3 as usize]),
        Vec3A::from(positions[v4 as usize]),
    );

    // Split the quad along the shorter diagonal
    let quad = if pos1_3d.distance_squared(pos4_3d) < pos2_3d.distance_squared(pos3_3d) {
        if negative_face { [v1, v4, v2, v1, v3, v4] } else { [v1, v2, v4, v1, v4, v3] }
    } else if negative_face {
        [v2, v3, v4, v2, v1, v3]
    } else {
        [v2, v4, v3, v2, v3, v1]
    };

    indices.extend_from_slice(&quad);
}

const CUBE_CORNERS: [[u32; 3]; 8] =
    [[0, 0, 0], [1, 0, 0], [0, 1, 0], [1, 1, 0], [0, 0, 1], [1, 0, 1], [0, 1, 1], [1, 1, 1]];

const CUBE_CORNER_VECTORS: [Vec3A; 8] = [
    Vec3A::from_array([0.0, 0.0, 0.0]),
    Vec3A::from_array([1.0, 0.0, 0.0]),
    Vec3A::from_array([0.0, 1.0, 0.0]),
    Vec3A::from_array([1.0, 1.0, 0.0]),
    Vec3A::from_array([0.0, 0.0, 1.0]),
    Vec3A::from_array([1.0, 0.0, 1.0]),
    Vec3A::from_array([0.0, 1.0, 1.0]),
    Vec3A::from_array([1.0, 1.0, 1.0]),
];

const CUBE_EDGES: [[u32; 2]; 12] = [
    [0b000, 0b001],
    [0b000, 0b010],
    [0b000, 0b100],
    [0b001, 0b011],
    [0b001, 0b101],
    [0b010, 0b011],
    [0b010, 0b110],
    [0b011, 0b111],
    [0b100, 0b101],
    [0b100, 0b110],
    [0b101, 0b111],
    [0b110, 0b111],
];

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_surface_net_generation() {
//         let mut voxels = Voxels::new();

//         // Create a simple 2x2x2 solid cube
//         for x in 0..2 {
//             for y in 0..2 {
//                 for z in 0..2 {
//                     voxels.set_voxel(IVec3::new(x, y, z), Voxel::Solid(1));
//                 }
//             }
//         }

//         let buffer = voxels.generate_surface_net_mesh(
//             IVec3::new(0, 0, 0),
//             IVec3::new(2, 2, 2),
//             1, // mesh voxel id
//         );

//         println!(
//             "Generated mesh with {} vertices and {} triangles",
//             buffer.positions.len(),
//             buffer.indices.len() / 3
//         );

//         assert!(!buffer.positions.is_empty());
//     }
// }
