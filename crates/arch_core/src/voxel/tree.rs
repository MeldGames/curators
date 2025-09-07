use crate::voxel::Voxel;
use bevy::prelude::*;

pub const TREE_ARY: usize = 4;
pub const TREE_LENGTH: usize = TREE_ARY * TREE_ARY * TREE_ARY;
pub const TREE_ARY_IVEC: IVec3 = IVec3::splat(TREE_ARY as i32);

pub const CHUNK_WIDTH: usize = 16;
pub const CHUNK_LENGTH: usize = CHUNK_WIDTH * CHUNK_WIDTH * CHUNK_WIDTH;

pub fn leaf_point(point: IVec3) -> IVec3 {
    point.div_euclid(IVec3::splat(CHUNK_WIDTH as i32))
}

pub fn to_leaf_index(relative_point: IVec3) -> usize {
    let IVec3 { x, y, z } = relative_point;
    z as usize + x as usize * CHUNK_WIDTH + y as usize * CHUNK_WIDTH * CHUNK_WIDTH
}

pub fn to_child_index(relative_point: IVec3) -> usize {
    let IVec3 { x, y, z } = relative_point;
    z as usize + x as usize * TREE_ARY + y as usize * TREE_ARY * TREE_ARY
}

#[derive(Clone, Debug)]
pub enum VoxelNode {
    /// Entire region is filled with a single voxel type.
    Solid(Voxel), // Solid(Voxel::Air) is the same as "Empty".
    Children(Box<[VoxelNode; TREE_LENGTH]>),
    /// Leaf node index
    Leaf(Box<[Voxel; CHUNK_LENGTH]>),
}

pub struct VoxelTree {
    pub root: VoxelNode,
    pub layers: usize, // how many layers down this tree goes
}

impl VoxelTree {
    pub fn new() -> VoxelTree {
        // TODO: is 1 the starting layer good? or should it be 0 indexed?
        Self { root: VoxelNode::Solid(Voxel::Air), layers: 1 }
    }

    pub fn grow_layer(&mut self) {
        let mut children: [VoxelNode; TREE_LENGTH] =
            std::array::from_fn(|_| VoxelNode::Solid(Voxel::Air));
        std::mem::swap(&mut children[0], &mut self.root);

        self.root = VoxelNode::Children(Box::new(children));
        self.layers += 1;
    }

    pub fn grow_n_layers(&mut self, layers: usize) {
        for _ in 0..layers {
            self.grow_layer();
        }
    }

    pub fn get_voxel(&self, voxel_point: IVec3) -> Voxel {
        let leaf_point = leaf_point(voxel_point);
        match self.get_leaf(leaf_point) {
            VoxelNode::Solid(voxel) => *voxel,
            VoxelNode::Leaf(leaf) => {
                let relative_voxel_point = leaf_point.rem_euclid(IVec3::splat(CHUNK_WIDTH as i32));
                let voxel_index = to_leaf_index(relative_voxel_point);
                if voxel_index > CHUNK_LENGTH {
                    warn!("voxel index out of bounds");
                    Voxel::Barrier
                } else {
                    leaf[voxel_index]
                }
            },
            VoxelNode::Children(_) => panic!("shouldn't end traversal on children"),
        }
    }

    pub fn get_leaf(&self, leaf_point: IVec3) -> &VoxelNode {
        self.get_leaf_recursive(self.layers, leaf_point, &self.root)
    }

    pub fn get_leaf_recursive<'a>(
        &self,
        layer: usize,
        leaf_point: IVec3,
        current_node: &'a VoxelNode,
    ) -> &'a VoxelNode {
        match current_node {
            VoxelNode::Children(children) => {
                let ary_point = leaf_point.div_euclid(TREE_ARY_IVEC);
                let relative_point = leaf_point.rem_euclid(TREE_ARY_IVEC);
                let child_index = to_child_index(relative_point);
                let next_node = &children[child_index];
                self.get_leaf_recursive(layer + 1, leaf_point - ary_point, next_node)
            },
            node @ VoxelNode::Leaf(_) | node @ VoxelNode::Solid(_) => node,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bevy::prelude::*;

    #[test]
    pub fn get_leaf() {
        let mut tree = VoxelTree::new();
        tree.grow_n_layers(3);

        let leaf = tree.get_leaf(IVec3::new(0, 0, 0));
        println!("leaf: {:?}", leaf);
    }
}
