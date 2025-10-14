use bevy::prelude::*;

use crate::voxel::Voxel;
use crate::voxel::simulation::data::SimChunk;

// 16^3 chunk encoded in run lengths.
#[derive(Clone, Debug, Reflect)]
pub struct RLEChunk {
    pub runs: Vec<(Voxel, u16)>,
}

impl RLEChunk {
    pub fn new() -> Self {
        Self { runs: Vec::new() }
    }

    #[inline]
    pub fn get_voxel(&self, relative_point: IVec3) -> Voxel {
        self.get_voxel_from_index(crate::voxel::simulation::data::linearize(relative_point))
    }

    #[inline]
    pub fn set_voxel(&mut self, relative_point: IVec3, voxel: Voxel) {
        let voxel_index = crate::voxel::simulation::data::linearize(relative_point);
        let run_index = self.run_index(voxel_index);
    }

    #[inline]
    pub fn run_index(&self, voxel_index: usize) -> usize {
        let mut count: usize = 0;
        for (run_index, (_run_voxel, run_count)) in self.runs.iter().enumerate() {
            count += *run_count as usize;
            if voxel_index < count {
                return run_index;
            }
        }

        panic!("voxel_index out of bounds: {:?}", voxel_index);
    }

    #[inline]
    pub fn get_voxel_from_index(&self, voxel_index: usize) -> Voxel {
        let run_index = self.run_index(voxel_index);
        let (voxel, _) = self.runs[run_index];
        voxel
    }

    pub fn from_sim(sim: &SimChunk) -> Self {
        let mut rle = RLEChunk::new();

        // for now read from sim chunk in a linearized fashion
        let mut iter = sim.voxels.iter().enumerate();
        let mut run = iter.next().map(|(_, voxel_packed)| *voxel_packed).unwrap();
        let mut run_count = 1;

        for (_voxel_index, voxel) in iter {
            if *voxel == run {
                run_count += 1;
            } else {
                rle.runs.push((run, run_count));
                run = *voxel;
                run_count = 1;
            }
        }

        rle.runs.push((run, run_count));

        // TODO: read from sim chunk and translate into a morton/z order curve
        rle
    }

    /*
    pub fn to_sim(&self) -> SimChunk {
        let mut chunk = SimChunk::new();
        let mut voxel_index = 0;
        for (run, run_count) in &self.runs {
            for _ in 0..*run_count {
                if cfg!(feature = "safe-bounds") {
                    chunk.voxels[voxel_index] = *run;
                } else {
                    unsafe {
                        *chunk.voxels.get_unchecked_mut(voxel_index) = *run;
                    }
                }
                voxel_index += 1;
            }
        }
        chunk
    }
    */

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

    /*
    #[test]
    pub fn sanity() {
        let mut sim_chunk = SimChunk::new();
        sim_chunk.voxels[linearize(ivec3(1, 1, 1))] = Voxel::Dirt;

        let rle = RLEChunk::from_sim(&sim_chunk);
        let from_rle = rle.to_sim();

        println!("runs: {:?}", rle.runs);
        assert_eq!(sim_chunk, from_rle);
    }
    */
}
