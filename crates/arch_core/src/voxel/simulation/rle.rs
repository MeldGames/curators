use bevy::prelude::*;

use crate::voxel::Voxel;
use crate::voxel::simulation::data::SimChunk;

pub struct RLEChunk {
    pub runs: Vec<(Voxel, u16)>,
}

impl RLEChunk {
    pub fn new() -> Self {
        Self { runs: Vec::new() }
    }

    pub fn from_sim(sim: &SimChunk) -> Self {
        let mut rle = RLEChunk::new();

        // for now read from sim chunk in a linearized fashion
        let mut iter = sim.voxels.iter().enumerate();
        let mut run = iter.next().map(|(_, voxel_packed)| Voxel::from_data(*voxel_packed)).unwrap();
        let mut run_count = 1;

        for (_voxel_index, voxel_packed) in iter {
            let voxel = Voxel::from_data(*voxel_packed);

            if voxel == run {
                run_count += 1;
            } else {
                rle.runs.push((run, run_count));
                run = voxel;
                run_count = 1;
            }
        }

        rle.runs.push((run, run_count));

        // TODO: read from sim chunk and translate into a morton/z order curve
        rle
    }

    pub fn to_sim(&self) -> SimChunk {
        let mut chunk = SimChunk::new();
        let mut voxel_index = 0;
        for (run, run_count) in &self.runs {
            for _ in 0..*run_count {
                if cfg!(feature = "safe-bounds") {
                    chunk.voxels[voxel_index] = run.data();
                } else {
                    unsafe {
                        *chunk.voxels.get_unchecked_mut(voxel_index) = run.data();
                    }
                }
                voxel_index += 1;
            }
        }
        chunk
    }

    pub fn runs_count(&self) -> usize {
        self.runs.len()
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::voxel::Voxel;
    use crate::voxel::simulation::data::{SimChunk, delinearize, linearize};
    use crate::voxel::simulation::rle::RLEChunk;

    #[test]
    pub fn sanity() {
        let mut sim_chunk = SimChunk::new();
        sim_chunk.voxels[linearize(ivec3(1, 1, 1))] = Voxel::Dirt.data();

        let rle = RLEChunk::from_sim(&sim_chunk);
        let from_rle = rle.to_sim();

        println!("runs: {:?}", rle.runs);
        assert_eq!(sim_chunk, from_rle);
    }
}
