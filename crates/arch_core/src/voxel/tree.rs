use crate::voxel::{Voxel, Voxels};
use bevy::{platform::collections::HashSet, prelude::*};
use std::fmt::{self, Debug, Formatter};

pub const TREE_ARY: usize = 4;
pub const TREE_LENGTH: usize = TREE_ARY * TREE_ARY * TREE_ARY;
pub const TREE_ARY_IVEC: IVec3 = IVec3::splat(TREE_ARY as i32);
pub const TREE_ARY_ILOG2: usize = TREE_ARY.ilog2() as usize;

pub const CHUNK_WIDTH: usize = 16;
pub const CHUNK_LENGTH: usize = CHUNK_WIDTH * CHUNK_WIDTH * CHUNK_WIDTH;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, draw_tree);
    app.add_systems(FixedLast, compress_tree);
}

pub fn draw_tree(grids: Query<(&Voxels,)>, mut gizmos: Gizmos) {
    for (voxels,) in &grids {
        voxels.tree.root.draw_gizmo(IVec3::ZERO, &mut gizmos);
    }
}

pub fn compress_tree(mut grids: Query<(&mut Voxels,)>) {
    for (mut voxels,) in &mut grids {
        voxels.tree.root.compress();
    }
}


/// Get the width of a region at a specific layer including the leaf chunk width.
#[inline]
pub const fn layer_width_voxel(layer: usize) -> usize {
    layer_width_chunk(layer) * CHUNK_WIDTH
}

/// Get the width of a region at a specific layer excluding the leaf chunk width.
#[inline]
pub const fn layer_width_chunk(layer: usize) -> usize {
    // 2^(log2(TREE_ARY) * layer)
    1 << (TREE_ARY_ILOG2 * layer)
}

/// Given a voxel point, find minimum position of the region the voxel point is in.
/// 
/// This is useful for finding the relative position of the voxel to the region and then
/// the relative position of the subdivided regions from this region.
#[inline]
pub fn layer_min_from_voxel(layer: usize, voxel_point: IVec3) -> IVec3 {
    let size = IVec3::splat(layer_width_voxel(layer) as i32);
    (voxel_point / size) * size
}

#[inline]
pub fn layer_min_from_chunk(layer: usize, chunk_point: IVec3) -> IVec3 {
    let size = IVec3::splat(layer_width_chunk(layer) as i32);
    (chunk_point / size) * size
}

/// Get the index into the leaf's voxel storage.
#[inline]
pub fn to_leaf_index(relative_leaf_point: IVec3) -> usize {
    assert!(relative_leaf_point.max_element() < CHUNK_WIDTH as i32);

    let IVec3 { x, y, z } = relative_leaf_point;
    z as usize + x as usize * CHUNK_WIDTH + y as usize * CHUNK_WIDTH * CHUNK_WIDTH
}

/// Get the index to the [`VoxelNode::Children`]'s subdivided region
/// 
/// Valid values are (0..[`TREE_ARY`], 0..[`TREE_ARY`], 0..[`TREE_ARY`]) non-inclusive.
#[inline]
pub fn to_child_index(relative_ary_point: IVec3) -> usize {
    assert!(relative_ary_point.max_element() < TREE_ARY as i32);

    let IVec3 { x, y, z } = relative_ary_point;
    z as usize + x as usize * TREE_ARY + y as usize * TREE_ARY * TREE_ARY
}

#[inline]
pub fn from_child_index(child_index: usize) -> IVec3 {
    assert!(child_index < TREE_LENGTH);

    let z = child_index % TREE_ARY;
    let x = (child_index / TREE_ARY) % TREE_ARY;
    let y = child_index / (TREE_ARY * TREE_ARY);
    IVec3::new(x as i32, y as i32, z as i32)
}

#[inline]
pub fn get_sublayer_index_from_voxel(layer: usize, voxel_point: IVec3) -> usize {
    let layer_min = layer_min_from_voxel(layer, voxel_point);
    let relative_voxel_point = voxel_point - layer_min;

    let subregion = relative_voxel_point / IVec3::splat(layer_width_voxel(layer - 1) as i32);
    // println!("layer: {:?}, relative_voxel_point: {:?}, subregion: {:?}", layer, relative_voxel_point, subregion);
    let sublayer_index = to_child_index(subregion);
    sublayer_index
}

#[inline]
pub fn get_sublayer_index_from_chunk(layer: usize, chunk_point: IVec3) -> usize {
    let layer_min = layer_min_from_chunk(layer, chunk_point);
    let relative_voxel_point = chunk_point - layer_min;

    let subregion = relative_voxel_point / IVec3::splat(layer_width_chunk(layer - 1) as i32);
    // println!("layer: {:?}, relative_voxel_point: {:?}, subregion: {:?}", layer, relative_voxel_point, subregion);
    let sublayer_index = to_child_index(subregion);
    sublayer_index
}


#[derive(Clone)]
pub enum VoxelNode {
    /// Entire region is filled with a single voxel type.
    Solid {
        layer: usize,
        voxel: Voxel, // Solid(Voxel::Air) is the same as "Empty".
    }, 
    /// Subdivided region, but not the bottom of the graph.
    Children {
        layer: usize,
        children: Box<[VoxelNode; TREE_LENGTH]>
    },
    /// Leaf node/bottom of the graph, holds fine-grain voxel data.
    Leaf {
        // layer is assumed 0
        leaf: Box<[Voxel; CHUNK_LENGTH]>,
    },
}

impl Debug for VoxelNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Solid { layer, voxel } => {
                write!(f, "Sl{:?}{:?}", layer, voxel)
                // f.debug_struct("Solid").field("layer", layer).field("voxel", voxel).finish()
            },
            Self::Children { layer, children } => {
                write!(f, "Cl{:?}[\n{:?}\n]", layer, children)
                // f.debug_struct("Children").field("layer", layer).field("children", children).finish()
            },
            Self::Leaf { leaf } => {
                let mut rle_debug = Vec::new();
                let mut run = leaf[0];
                let mut run_count = 0;
                for voxel in leaf.iter() {
                    if run == *voxel {
                        run_count += 1;
                    } else {
                        rle_debug.push(format!("{:?} {:?}", run, run_count));
                        run = *voxel;
                        run_count = 1;
                    }
                }

                rle_debug.push(format!("{:?} {:?}", run, run_count));
                write!(f, "L[{}]", rle_debug.join(", "))
                // f.debug_struct("Leaf").field("leaf", leaf).finish()
            },
        }
    }
}

impl VoxelNode {
    pub fn draw_gizmo(&self, origin: IVec3, gizmos: &mut Gizmos) {
        match self {
            Self::Solid { layer, .. }=> {
                let width = layer_width_voxel(*layer) as i32;
                gizmos.cuboid(Transform {
                    translation: (origin.as_vec3() + Vec3::splat(width as f32) / 2.0) * crate::voxel::GRID_SCALE,
                    scale: Vec3::splat(width as f32) * crate::voxel::GRID_SCALE,
                    ..default()
                }, Color::srgb(1.0, 0.0, 0.0));
            }
            Self::Children { layer, children } => {
                for (child_index, child) in children.iter().enumerate() {
                    let child_position = from_child_index(child_index);
                    let voxel_space_offset = child_position * layer_width_voxel(*layer - 1) as i32;
                    child.draw_gizmo(origin + voxel_space_offset, gizmos);
                }
            }
            Self::Leaf { .. } => {
                let width = layer_width_voxel(0) as i32;
                gizmos.cuboid(Transform {
                    translation: (origin.as_vec3() + Vec3::splat(width as f32) / 2.0) * crate::voxel::GRID_SCALE,
                    scale: Vec3::splat(width as f32) * crate::voxel::GRID_SCALE,
                    ..default()
                }, Color::srgb(1.0, 0.0, 0.0));
            }
        }
    }

    pub fn renderable_chunks(&self, origin: IVec3, buffer: &mut Vec<IVec3>) {
        match self {
            Self::Solid {
                layer, voxel
            } => {
                if !voxel.rendered() {
                    return;
                }

                // TODO: Only add the chunks on the surface of this solid region to the list.

                // add every chunk point from this layer 
                let width = layer_width_chunk(*layer) as i32;
                for x in 0..width {
                    for y in 0..width {
                        for z in 0..width {
                            buffer.push(origin + IVec3::new(x, y, z));
                        }
                    }
                }
            }
            Self::Leaf { .. } => { // assume there is a renderable voxel in this leaf
                buffer.push(origin);
            }
            Self::Children { layer, children } => {
                for (child_index, child) in children.iter().enumerate() {
                    let child_position = from_child_index(child_index);
                    let chunk_space_offset = child_position * layer_width_chunk(*layer) as i32;
                    child.renderable_chunks(origin + chunk_space_offset, buffer);
                }
            }
        }
    }

    pub fn subdivide(&mut self) {
        // layer 0 subdivision is a leaf
        // layer 1+ subdivision are children
        match self {
            Self::Solid {
                layer,
                voxel,
            } => {
                if *layer == 0 {
                    *self = Self::Leaf {
                        leaf: Box::new([*voxel; CHUNK_LENGTH]),
                    };
                } else {
                    *self = Self::Children {
                        layer: *layer,
                        children: Box::new(std::array::from_fn(|_| VoxelNode::Solid {
                            layer: *layer - 1,
                            voxel: *voxel,
                        }))
                    };
                }
            }
            _ => panic!("Tried to fracture a non-solid VoxelNode"),
            // Self::Leaf { .. } | Self::Children { .. } => {} // already fractured
        }
    }

    /// Compress into a Solid node if all of the voxels are the same.
    pub fn compress(&mut self) {
        match self {
            Self::Children { layer, children } => {
                children[0].compress();

                let mut all_solid = true;
                let solid_voxel = match children[0] {
                    Self::Solid { voxel, .. } => {
                        voxel
                    }
                    _ => {
                        all_solid = false;
                        Voxel::Air
                    },
                };

                for child in children.iter_mut().skip(1) {
                    child.compress();
                    match child {
                        Self::Solid { voxel, .. } => {
                            if *voxel != solid_voxel {
                                all_solid = false;
                            }
                        }
                        _ => all_solid = false,
                    }
                }

                if all_solid {
                    *self = Self::Solid { layer: *layer, voxel: solid_voxel };
                }
            }
            Self::Leaf { leaf } => {
                let first_voxel = leaf[0];
                for voxel in leaf.iter().skip(1) {
                    if *voxel != first_voxel {
                        return;
                    }
                }

                // we are all the same voxel, compress to solid
                *self = Self::Solid { layer: 0, voxel: first_voxel };
            }
            Self::Solid { .. } => {}, // already compressed
        }
    }

    pub fn get_voxel(&self, voxel_point: IVec3) -> Voxel {
        match self {
            // traverse downwards
            VoxelNode::Children { layer, children } => {
                let sublayer_index = get_sublayer_index_from_voxel(*layer, voxel_point);
                let next_node = &children[sublayer_index];
                next_node.get_voxel(voxel_point)
            },
            VoxelNode::Solid { voxel, .. } => *voxel,
            VoxelNode::Leaf { leaf } => {
                let relative_voxel_point = voxel_point.rem_euclid(IVec3::splat(CHUNK_WIDTH as i32));
                let voxel_index = to_leaf_index(relative_voxel_point);
                if voxel_index > CHUNK_LENGTH {
                    panic!("voxel index out of bounds");
                } else {
                    leaf[voxel_index]
                }
            }
        }
    }

    /// Set a specific voxel at the lowest layer of the voxel tree.
    /// This will fracture any [`VoxelNode::Solid`] nodes on the way down.
    pub fn set_voxel(
        &mut self,
        voxel_point: IVec3,
        voxel: Voxel,
    ) -> bool {
        match self {
            // traverse downwards
            VoxelNode::Children { layer, children } => {
                let sublayer_index = get_sublayer_index_from_voxel(*layer, voxel_point);
                let next_node = &mut children[sublayer_index];
                next_node.set_voxel(voxel_point, voxel)
            },
            // fracture into child or leaf
            solid @ VoxelNode::Solid { .. } => {
                // if this solid region is already the voxel we want to set, just exit.
                match solid {
                    VoxelNode::Solid { voxel: solid_voxel, .. } => {
                        if *solid_voxel == voxel {
                            return false;
                        }
                    }
                    _ => unreachable!(),
                }

                solid.subdivide();
                let subdivided = solid;
                subdivided.set_voxel(voxel_point, voxel) // recurse into the correct path
            }
            VoxelNode::Leaf { leaf } => {
                let relative_voxel_point = voxel_point.rem_euclid(IVec3::splat(CHUNK_WIDTH as i32));
                let voxel_index = to_leaf_index(relative_voxel_point);
                if voxel_index > CHUNK_LENGTH {
                    warn!("voxel index out of bounds");
                    false
                } else {
                    leaf[voxel_index] = voxel;
                    true
                }
            }
        }
    }

    /// Get the contents of a bottom level chunk
    pub fn get_chunk<'a>(
        &'a self,
        chunk_point: IVec3, // voxel point divided by chunk/leaf size
    ) -> &'a VoxelNode {
        match self {
            VoxelNode::Children { layer, children } => {
                assert!(*layer > 0);

                let sublayer_index = get_sublayer_index_from_chunk(*layer, chunk_point);
                // println!("sublayer_index: {:?}", sublayer_index);
                let next_node = &children[sublayer_index];
                next_node.get_chunk(chunk_point)
            },
            leaf @ VoxelNode::Leaf { .. } => leaf,
            solid @ VoxelNode::Solid { .. } => solid,
        }
    }

    pub fn get_chunk_mut<'a>(
        &'a mut self,
        chunk_point: IVec3, // voxel point divided by chunk/leaf size
    ) -> &'a mut VoxelNode {
        match self {
            VoxelNode::Children { layer, children } => {
                assert!(*layer > 0);

                let sublayer_index = get_sublayer_index_from_chunk(*layer, chunk_point);
                // println!("sublayer_index: {:?}", sublayer_index);
                let next_node = &mut children[sublayer_index];
                next_node.get_chunk_mut(chunk_point)
            },
            leaf @ VoxelNode::Leaf { .. } => leaf,
            solid @ VoxelNode::Solid { .. } => solid,
        }
    }

    pub fn set_chunk_data<'a>(
        &'a mut self,
        chunk_point: IVec3, // voxel point divided by chunk/leaf size
        chunk_data: [Voxel; CHUNK_LENGTH],
    ) {
        match self {
            // traverse downwards
            VoxelNode::Children { layer, children } => {
                let sublayer_index = get_sublayer_index_from_chunk(*layer, chunk_point);
                let next_node = &mut children[sublayer_index];
                next_node.set_chunk_data(chunk_point, chunk_data);
            },
            // fracture into child or leaf
            solid @ VoxelNode::Solid { .. } => {
                solid.subdivide();
                let subdivided = solid;
                subdivided.set_chunk_data(chunk_point, chunk_data); // recurse into the correct path
            }
            VoxelNode::Leaf { leaf } => {
                **leaf = chunk_data;
            }
        }
    }

    pub fn is_subdivided(&self) -> bool {
        match self {
            VoxelNode::Solid { .. } => false,
            VoxelNode::Leaf { .. } | VoxelNode::Children { .. } => true,
        }
    }

    pub fn is_solid(&self) -> bool {
        match self {
            VoxelNode::Solid { .. } => true,
            VoxelNode::Leaf { .. } | VoxelNode::Children { .. } => false,
        }
    }

    pub fn layer(&self) -> usize {
        match self {
            VoxelNode::Solid { layer, ..} => *layer,
            VoxelNode::Children { layer, ..} => *layer,
            VoxelNode::Leaf { .. }  => 0,
        }
    }

    pub fn voxel_width(&self) -> usize {
        layer_width_voxel(self.layer())
    }

    pub fn chunk_width(&self) -> usize {
        layer_width_chunk(self.layer())
    }
}

#[derive(Clone, Debug)]
pub struct VoxelTree {
    pub root: VoxelNode,
    pub changed_chunks: HashSet<IVec3>,
}

impl VoxelTree {
    pub fn new() -> VoxelTree {
        // TODO: is 1 the starting layer good? or should it be 0 indexed?
        Self { root: VoxelNode::Solid { layer: 0, voxel: Voxel::Air }, changed_chunks: default(), }
    }

    pub fn root_layer(&self) -> usize {
        self.root.layer()
    }

    pub fn grow_layer(&mut self) {
        let root_layer = self.root_layer();

        // take the root with a temporary value
        // let root = std::mem::swap(self.root, 

        // leaf -> children [leaf, ..]
        // children -> children [children, ..]
        // solid -> solid
        let new_root = match &self.root {
            VoxelNode::Leaf { .. } | VoxelNode::Children { .. } => {
                let mut children: [VoxelNode; TREE_LENGTH] =
                    std::array::from_fn(|_| VoxelNode::Solid { layer: root_layer, voxel: Voxel::Air });
                std::mem::swap(&mut children[0], &mut self.root);
                VoxelNode::Children { layer: root_layer + 1, children: Box::new(children) }
            }
            VoxelNode::Solid { layer, voxel } => {
                VoxelNode::Solid { layer: *layer + 1, voxel: *voxel }
            }
        };

        self.root = new_root;
    }

    pub fn grow_n_layers(&mut self, layers: usize) {
        for _ in 0..layers {
            self.grow_layer();
        }
    }

    pub fn get_voxel(&self, voxel_point: IVec3) -> Voxel {
        if !self.voxel_point_in_bounds(voxel_point) {
            return Voxel::Barrier;
        }

        self.root.get_voxel(voxel_point)
    }
    
    pub fn set_voxel(&mut self, voxel_point: IVec3, voxel: Voxel) {
        if !self.voxel_point_in_bounds(voxel_point) {
            warn!("voxel point set out-of-bounds: {:?} {:?}", voxel_point, voxel);
            return;
        }

        if self.root.set_voxel(voxel_point, voxel) {
            self.changed_chunks.insert(voxel_point / IVec3::splat(CHUNK_WIDTH as i32));
        }
    }

    pub fn set_chunk_data(&mut self, chunk_point: IVec3, chunk_data: [Voxel; 4096]) {
        assert!(self.chunk_point_in_bounds(chunk_point));
        self.root.set_chunk_data(chunk_point, chunk_data);

        // all neighbors should re-mesh
        for x in -1..1 {
            for y in -1..1 {
                for z in -1..1 {
                    let offset = IVec3::new(x, y, z);
                    self.changed_chunks.insert(chunk_point + offset);
                }
            }
        }
    }

    pub fn get_chunk_mut(&mut self, chunk_point: IVec3) -> &mut VoxelNode {
        assert!(self.chunk_point_in_bounds(chunk_point));
        self.changed_chunks.insert(chunk_point);
        self.root.get_chunk_mut(chunk_point)
    }

    pub fn voxel_point_in_bounds(&self, voxel_point: IVec3) -> bool {
        voxel_point.max_element() < self.root.voxel_width() as i32 && voxel_point.min_element() >= 0
    }

    pub fn chunk_point_in_bounds(&self, chunk_point: IVec3) -> bool {
        chunk_point.max_element() < self.root.chunk_width() as i32 && chunk_point.min_element() >= 0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bevy::prelude::*;

    #[test]
    pub fn get_set_sanity() {
        let mut tree = VoxelTree::new();
        tree.grow_n_layers(2);
        eprintln!("initial tree: {:?}", tree);
        eprintln!("tree size: {:?}", tree.root.voxel_width());

        const N: i32 = 4 * 4 * 16;
        let mut iter = 0;
        for x in 0..N {
            for y in 0..N {
                for z in 0..N {
                    if iter % 1_000_000 == 0 { // just for larger scale tests
                        println!("iter: {:?}/{:?}", iter, N * N * N);
                    }
                    iter += 1;

                    let voxel_point = IVec3::new(x, y, z);
                    // eprintln!("point: {:?}", voxel_point);
                    // println!("{:?}", tree);
                    let voxel = tree.get_voxel(voxel_point);
                    assert_eq!(voxel, Voxel::Air);
                    tree.root.set_voxel(voxel_point, Voxel::Dirt);
                    let voxel = tree.get_voxel(voxel_point);
                    assert_eq!(voxel, Voxel::Dirt);
                }
            }
        }

        eprintln!("{:?}", tree);
        tree.root.compress();
        eprintln!("compressed: {:?}", tree);
    }

    #[test]
    pub fn layer_remainder() {
        let voxel_point = IVec3::splat(6000);
        // let voxel_point = IVec3::new(8, 8, 8);
        for layer in 0..6 {
            println!("width: {:?}", layer_width_voxel(layer));
            println!("min: {:?}", layer_min_from_voxel(layer, voxel_point));
            println!("relative_voxel: {:?}", voxel_point - layer_min_from_voxel(layer, voxel_point));
            println!("layer_in: {:?}", (voxel_point - layer_min_from_voxel(layer, voxel_point)).rem_euclid(IVec3::splat(TREE_ARY as i32)));
        }
    }

    // #[test]
    // pub fn compress() {
    //     let mut tree = VoxelTree::new();
    //     tree.grow_n_layers(1);

    //     const N: i32 = 32;
    //     for x in 0..N {
    //         for y in 0..N {
    //             for z in 0..N {
    //                 let voxel_point = IVec3::new(x, y, z);
    //                 // eprintln!("point: {:?}", voxel_point);
    //                 // println!("{:?}", tree);
    //                 let voxel = tree.get_voxel(voxel_point);
    //                 assert_eq!(voxel, Voxel::Air);
    //                 tree.root.set_voxel(voxel_point, Voxel::Dirt);
    //                 let voxel = tree.get_voxel(voxel_point);
    //                 assert_eq!(voxel, Voxel::Dirt);
    //             }
    //         }
    //     }

    //     tree.root.compress();
    // }
}
