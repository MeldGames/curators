use bevy::prelude::*;

use crate::voxel::simulation::data::SimChunk;

pub struct ChunkView<'a, const SIZE: usize> {
    pub chunks: [&'a mut SimChunk; SIZE * SIZE * SIZE],
}

impl<'a, const SIZE: usize> ChunkView<'a, SIZE> {
    pub fn simulate(&mut self) {
        let mut chunk_index = 0;
        let mut voxel_index = 0;

        for chunk in self.chunks {}
    }
}
