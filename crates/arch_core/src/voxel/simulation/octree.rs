pub enum OctreeNode {
    Nodes([OctreeNode; 8]),
    Chunk(usize),
}

pub struct Octree {
    pub chunks: Vec<SimChunk>,
    pub free_indices: Vec<usize>,
    pub indices: OctreeNode,
}

impl Octree {
    pub fn new() -> Self {
        Self { chunks: Vec::new(), free_indices: Vec::new() }
    }
}
