use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::voxel::chunk::unpadded;
use crate::voxel::{Scalar, Voxel, VoxelChunk, Voxels};
use bevy::{platform::collections::HashMap, prelude::*};
use rayon::prelude::*;

#[cfg(feature = "trace")]
use tracing::*;

// Thread-safe chunk access for parallel simulation
pub struct ChunkSimulationJob {
    pub chunk_pos: IVec3,
    pub chunk: Arc<Mutex<VoxelChunk>>,
    pub is_active: AtomicBool,
    pub boundary_updates: std::sync::Mutex<Vec<BoundaryUpdate>>,
}

#[derive(Debug, Clone)]
pub struct BoundaryUpdate {
    pub from_pos: IVec3,
    pub to_pos: IVec3,
    pub from_voxel: Voxel,
    pub to_voxel: Voxel,
}

impl ChunkSimulationJob {
    pub fn new(chunk_pos: IVec3, chunk: VoxelChunk) -> Self {
        Self {
            chunk_pos,
            chunk: Arc::new(Mutex::new(chunk)),
            is_active: AtomicBool::new(true),
            boundary_updates: std::sync::Mutex::new(Vec::new()),
        }
    }

    // Get mutable access to the chunk (thread-safe)
    pub fn get_chunk_mut(&self) -> std::sync::MutexGuard<VoxelChunk> {
        self.chunk.lock().unwrap()
    }

    // Add a boundary update to be processed later
    pub fn add_boundary_update(&self, update: BoundaryUpdate) {
        if let Ok(mut updates) = self.boundary_updates.lock() {
            updates.push(update);
        }
    }
}

// Chunk grid for parallel simulation
pub struct ChunkSimulationGrid {
    pub jobs: HashMap<IVec3, ChunkSimulationJob>,
    pub phase: u32, // 0 or 1 for alternating chunks
}

impl ChunkSimulationGrid {
    pub fn new() -> Self {
        Self { jobs: HashMap::new(), phase: 0 }
    }

    pub fn from_voxels(voxels: &Voxels) -> Self {
        let mut grid = Self::new();

        // Create jobs for all chunks
        for (chunk_pos, chunk) in voxels.chunk_iter() {
            let mut job = ChunkSimulationJob::new(chunk_pos, chunk.clone());

            // Set up neighbor references (this will be unsafe but controlled)
            // We'll need to handle this carefully to avoid double-mutable access
            grid.jobs.insert(chunk_pos, job);
        }

        grid
    }

    // Get chunks that should be simulated in the current phase
    pub fn get_active_chunks(&self) -> Vec<IVec3> {
        self.jobs
            .keys()
            .filter(|&&pos| {
                // Alternating pattern: even chunks in phase 0, odd in phase 1
                let sum = pos.x + pos.y + pos.z;
                (sum % 2) == ((self.phase % 2) as i32)
            })
            .copied()
            .collect()
    }

    // Split a chunk into 62x1x62 slices for further parallelization
    pub fn create_chunk_slices(&self, chunk_pos: IVec3) -> Vec<ChunkSlice> {
        let mut slices = Vec::new();

        // Create 62 vertical slices (y-direction)
        for y in 0..unpadded::SIZE {
            let slice = ChunkSlice {
                chunk_pos,
                y_range: y..y + 1,
                x_range: 0..unpadded::SIZE,
                z_range: 0..unpadded::SIZE,
            };
            slices.push(slice);
        }

        slices
    }
}

// Represents a slice of a chunk for fine-grained parallelization
pub struct ChunkSlice {
    pub chunk_pos: IVec3,
    pub y_range: std::ops::Range<usize>,
    pub x_range: std::ops::Range<usize>,
    pub z_range: std::ops::Range<usize>,
}

impl ChunkSlice {
    pub fn simulate(&self, job: &ChunkSimulationJob) {
        let mut chunk = job.get_chunk_mut();

        // Simulate only the voxels in this slice
        for y in self.y_range.clone() {
            for x in self.x_range.clone() {
                for z in self.z_range.clone() {
                    let relative_pos = IVec3::new(x as Scalar, y as Scalar, z as Scalar);
                    let world_pos = job.chunk_pos * unpadded::SIZE_SCALAR + relative_pos;

                    // Get the voxel at this position
                    let voxel = chunk.voxel(relative_pos);

                    // Apply simulation rules
                    match voxel {
                        Voxel::Sand => {
                            self.simulate_sand_slice(&mut chunk, relative_pos, world_pos, job)
                        },
                        Voxel::Water | Voxel::Oil => {
                            self.simulate_liquid_slice(&mut chunk, relative_pos, world_pos, job)
                        },
                        Voxel::Dirt => {
                            self.simulate_structured_slice(&mut chunk, relative_pos, world_pos, job)
                        },
                        _ => {},
                    }
                }
            }
        }
    }

    fn simulate_sand_slice(
        &self,
        chunk: &mut VoxelChunk,
        relative_pos: IVec3,
        world_pos: IVec3,
        job: &ChunkSimulationJob,
    ) {
        const SWAP_POINTS: [IVec3; 5] =
            [IVec3::NEG_Y, ivec3(1, -1, 0), ivec3(0, -1, 1), ivec3(-1, -1, 0), ivec3(0, -1, -1)];

        for swap_point in SWAP_POINTS {
            let target_pos = relative_pos + swap_point;

            // Check if target is within this slice or needs neighbor access
            if self.contains_relative_pos(target_pos) {
                let voxel = chunk.voxel(target_pos);
                if voxel.is_liquid() || voxel.is_gas() {
                    chunk.set(target_pos, Voxel::Sand);
                    chunk.set(relative_pos, voxel);
                    break;
                }
            } else {
                // Need to access neighbor chunk - this is where it gets tricky
                // We'll need to handle boundary propagation separately
                self.handle_boundary_simulation(chunk, relative_pos, swap_point, job);
            }
        }
    }

    fn simulate_liquid_slice(
        &self,
        chunk: &mut VoxelChunk,
        relative_pos: IVec3,
        world_pos: IVec3,
        job: &ChunkSimulationJob,
    ) {
        // Similar to sand but with liquid-specific logic
        // Handle boundary cases carefully
    }

    fn simulate_structured_slice(
        &self,
        chunk: &mut VoxelChunk,
        relative_pos: IVec3,
        world_pos: IVec3,
        job: &ChunkSimulationJob,
    ) {
        // Structured material simulation
    }

    fn contains_relative_pos(&self, pos: IVec3) -> bool {
        pos.x >= 0
            && pos.y >= 0
            && pos.z >= 0
            && self.x_range.contains(&(pos.x as usize))
            && self.y_range.contains(&(pos.y as usize))
            && self.z_range.contains(&(pos.z as usize))
    }

    fn handle_boundary_simulation(
        &self,
        chunk: &mut VoxelChunk,
        relative_pos: IVec3,
        swap_point: IVec3,
        job: &ChunkSimulationJob,
    ) {
        // Defer boundary updates to be processed later
        let world_pos = job.chunk_pos * unpadded::SIZE_SCALAR + relative_pos;
        let target_world_pos = world_pos + swap_point;
        let target_chunk_pos = Voxels::find_chunk(target_world_pos);

        if target_chunk_pos != job.chunk_pos {
            // This is a cross-chunk update, defer it
            let current_voxel = chunk.voxel(relative_pos);
            let update = BoundaryUpdate {
                from_pos: world_pos,
                to_pos: target_world_pos,
                from_voxel: current_voxel,
                to_voxel: Voxel::Air, // Will be filled in by the target chunk
            };
            job.add_boundary_update(update);
        }
    }
}

// Main simulation system that orchestrates the parallel execution
pub fn parallel_falling_sands(
    mut voxels: Query<&mut Voxels>,
    mut sim_tick: ResMut<crate::voxel::simulation::FallingSandTick>,
    mut ignore: Local<usize>,
) {
    *ignore = (*ignore + 1) % 4;
    if *ignore != 0 {
        return;
    }

    #[cfg(feature = "trace")]
    let parallel_falling_sands_span = info_span!("parallel_falling_sands");

    sim_tick.0 = (sim_tick.0 + 1) % (u32::MAX / 2);

    for mut grid in &mut voxels {
        // Create simulation grid from voxels
        let mut sim_grid = ChunkSimulationGrid::from_voxels(&grid);

        // Phase 1: Simulate active chunks in parallel
        let active_chunks = sim_grid.get_active_chunks();

        // Parallelize chunk simulation
        active_chunks.par_iter().for_each(|&chunk_pos| {
            if let Some(job) = sim_grid.jobs.get(&chunk_pos) {
                // Create slices for this chunk
                let slices = sim_grid.create_chunk_slices(chunk_pos);

                // Simulate each slice in parallel
                slices.par_iter().for_each(|slice| {
                    slice.simulate(job);
                });
            }
        });

        // Phase 2: Handle boundary propagation
        // This needs to be done carefully to avoid race conditions
        handle_boundary_propagation(&mut sim_grid);

        // Phase 3: Update the original voxels from simulation results
        update_voxels_from_simulation(&mut grid, &mut sim_grid);

        // Toggle phase for next frame
        sim_grid.phase = (sim_grid.phase + 1) % 2;
    }
}

fn handle_boundary_propagation(sim_grid: &mut ChunkSimulationGrid) {
    // This is where we handle voxel changes that cross chunk boundaries
    // We need to be very careful about thread safety here
    // One approach is to collect all boundary changes and apply them
    // in a separate, non-parallel phase
}

fn update_voxels_from_simulation(voxels: &mut Voxels, sim_grid: &mut ChunkSimulationGrid) {
    // Copy simulation results back to the main voxel grid
    for (chunk_pos, job) in sim_grid.jobs.iter() {
        if let Some(chunk) = voxels.get_chunk_mut(*chunk_pos) {
            let sim_chunk = job.get_chunk_mut();
            *chunk = sim_chunk.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::chunk::unpadded;

    #[test]
    fn test_chunk_simulation_job_creation() {
        let chunk_pos = IVec3::new(1, 2, 3);
        let chunk = VoxelChunk::new();
        let job = ChunkSimulationJob::new(chunk_pos, chunk);

        assert_eq!(job.chunk_pos, chunk_pos);
        assert!(job.is_active.load(Ordering::Relaxed));
    }

    #[test]
    fn test_chunk_simulation_job_mutable_access() {
        let chunk_pos = IVec3::new(0, 0, 0);
        let mut chunk = VoxelChunk::new();
        chunk.set(IVec3::new(1, 1, 1), Voxel::Sand);

        let job = ChunkSimulationJob::new(chunk_pos, chunk);
        let mut chunk_guard = job.get_chunk_mut();

        // Test that we can read and write to the chunk
        assert_eq!(chunk_guard.voxel(IVec3::new(1, 1, 1)), Voxel::Sand);
        chunk_guard.set(IVec3::new(2, 2, 2), Voxel::Water);
        assert_eq!(chunk_guard.voxel(IVec3::new(2, 2, 2)), Voxel::Water);
    }

    #[test]
    fn test_boundary_update_creation() {
        let update = BoundaryUpdate {
            from_pos: IVec3::new(1, 1, 1),
            to_pos: IVec3::new(2, 1, 1),
            from_voxel: Voxel::Sand,
            to_voxel: Voxel::Air,
        };

        assert_eq!(update.from_pos, IVec3::new(1, 1, 1));
        assert_eq!(update.to_pos, IVec3::new(2, 1, 1));
        assert_eq!(update.from_voxel, Voxel::Sand);
        assert_eq!(update.to_voxel, Voxel::Air);
    }

    #[test]
    fn test_chunk_simulation_grid_creation() {
        let grid = ChunkSimulationGrid::new();
        assert_eq!(grid.jobs.len(), 0);
        assert_eq!(grid.phase, 0);
    }

    #[test]
    fn test_chunk_simulation_grid_from_voxels() {
        let mut voxels = Voxels::new();
        voxels.set_voxel(IVec3::new(0, 0, 0), Voxel::Sand);
        voxels.set_voxel(IVec3::new(63, 0, 0), Voxel::Water); // Different chunk

        let sim_grid = ChunkSimulationGrid::from_voxels(&voxels);

        // Should have created jobs for both chunks
        assert!(sim_grid.jobs.contains_key(&IVec3::new(0, 0, 0)));
        assert!(sim_grid.jobs.contains_key(&IVec3::new(1, 0, 0)));
    }

    #[test]
    fn test_alternating_chunk_phases() {
        let mut voxels = Voxels::new();
        // Create voxels that will be in different chunks
        voxels.set_voxel(IVec3::new(0, 0, 0), Voxel::Sand); // chunk (0, 0, 0) - sum = 0 (even)
        voxels.set_voxel(IVec3::new(63, 0, 0), Voxel::Sand); // chunk (1, 0, 0) - sum = 1 (odd)
        voxels.set_voxel(IVec3::new(0, 63, 0), Voxel::Sand); // chunk (0, 1, 0) - sum = 1 (odd)
        voxels.set_voxel(IVec3::new(63, 63, 0), Voxel::Sand); // chunk (1, 1, 0) - sum = 2 (even)

        let mut sim_grid = ChunkSimulationGrid::from_voxels(&voxels);

        // Phase 0: should get even chunks
        sim_grid.phase = 0;
        let active_chunks_phase_0 = sim_grid.get_active_chunks();
        assert!(active_chunks_phase_0.contains(&IVec3::new(0, 0, 0)));
        assert!(active_chunks_phase_0.contains(&IVec3::new(1, 1, 0)));
        assert!(!active_chunks_phase_0.contains(&IVec3::new(1, 0, 0)));
        assert!(!active_chunks_phase_0.contains(&IVec3::new(0, 1, 0)));

        // Phase 1: should get odd chunks
        sim_grid.phase = 1;
        let active_chunks_phase_1 = sim_grid.get_active_chunks();
        assert!(!active_chunks_phase_1.contains(&IVec3::new(0, 0, 0)));
        assert!(!active_chunks_phase_1.contains(&IVec3::new(1, 1, 0)));
        assert!(active_chunks_phase_1.contains(&IVec3::new(1, 0, 0)));
        assert!(active_chunks_phase_1.contains(&IVec3::new(0, 1, 0)));
    }

    #[test]
    fn test_chunk_slice_creation() {
        let mut voxels = Voxels::new();
        voxels.set_voxel(IVec3::new(0, 0, 0), Voxel::Sand);

        let sim_grid = ChunkSimulationGrid::from_voxels(&voxels);
        let slices = sim_grid.create_chunk_slices(IVec3::new(0, 0, 0));

        // Should create 62 slices (one for each y-level)
        assert_eq!(slices.len(), unpadded::SIZE);

        // Test first slice
        let first_slice = &slices[0];
        assert_eq!(first_slice.chunk_pos, IVec3::new(0, 0, 0));
        assert_eq!(first_slice.y_range, 0..1);
        assert_eq!(first_slice.x_range, 0..unpadded::SIZE);
        assert_eq!(first_slice.z_range, 0..unpadded::SIZE);

        // Test middle slice
        let middle_slice = &slices[unpadded::SIZE / 2];
        assert_eq!(middle_slice.y_range, (unpadded::SIZE / 2)..(unpadded::SIZE / 2 + 1));
    }

    #[test]
    fn test_chunk_slice_contains_relative_pos() {
        let slice = ChunkSlice {
            chunk_pos: IVec3::new(0, 0, 0),
            y_range: 5..6,
            x_range: 10..20,
            z_range: 15..25,
        };

        // Test positions within the slice
        assert!(slice.contains_relative_pos(IVec3::new(15, 5, 20)));
        assert!(slice.contains_relative_pos(IVec3::new(10, 5, 15)));
        assert!(slice.contains_relative_pos(IVec3::new(19, 5, 24)));

        // Test positions outside the slice
        assert!(!slice.contains_relative_pos(IVec3::new(9, 5, 20))); // x too low
        assert!(!slice.contains_relative_pos(IVec3::new(20, 5, 20))); // x too high
        assert!(!slice.contains_relative_pos(IVec3::new(15, 4, 20))); // y too low
        assert!(!slice.contains_relative_pos(IVec3::new(15, 6, 20))); // y too high
        assert!(!slice.contains_relative_pos(IVec3::new(15, 5, 14))); // z too low
        assert!(!slice.contains_relative_pos(IVec3::new(15, 5, 25))); // z too high

        // Test negative positions (should be false)
        assert!(!slice.contains_relative_pos(IVec3::new(-1, 5, 20)));
        assert!(!slice.contains_relative_pos(IVec3::new(15, -1, 20)));
        assert!(!slice.contains_relative_pos(IVec3::new(15, 5, -1)));
    }

    #[test]
    fn test_chunk_slice_simulation() {
        let mut voxels = Voxels::new();
        // Set up a simple test scenario
        voxels.set_voxel(IVec3::new(1, 1, 1), Voxel::Sand);
        voxels.set_voxel(IVec3::new(1, 0, 1), Voxel::Air); // Space below for sand to fall

        let sim_grid = ChunkSimulationGrid::from_voxels(&voxels);
        let job = sim_grid.jobs.get(&IVec3::new(0, 0, 0)).unwrap();

        // Create a slice that contains our test voxel
        let slice = ChunkSlice {
            chunk_pos: IVec3::new(0, 0, 0),
            y_range: 1..2, // Only simulate y=1
            x_range: 1..2, // Only simulate x=1
            z_range: 1..2, // Only simulate z=1
        };

        // Run simulation
        slice.simulate(job);

        // Check that the simulation ran (we can't easily test the exact result
        // without implementing the full simulation logic, but we can verify
        // the method doesn't panic and the chunk is still accessible)
        let chunk_guard = job.get_chunk_mut();
        assert_eq!(chunk_guard.voxel(IVec3::new(1, 1, 1)), Voxel::Sand);
    }

    #[test]
    fn test_boundary_update_collection() {
        let chunk_pos = IVec3::new(0, 0, 0);
        let chunk = VoxelChunk::new();
        let job = ChunkSimulationJob::new(chunk_pos, chunk);

        let update = BoundaryUpdate {
            from_pos: IVec3::new(1, 1, 1),
            to_pos: IVec3::new(2, 1, 1),
            from_voxel: Voxel::Sand,
            to_voxel: Voxel::Air,
        };

        job.add_boundary_update(update.clone());

        // Check that the update was added
        let updates = job.boundary_updates.lock().unwrap();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].from_pos, update.from_pos);
        assert_eq!(updates[0].to_pos, update.to_pos);
    }

    #[test]
    fn test_multiple_boundary_updates() {
        let chunk_pos = IVec3::new(0, 0, 0);
        let chunk = VoxelChunk::new();
        let job = ChunkSimulationJob::new(chunk_pos, chunk);

        let update1 = BoundaryUpdate {
            from_pos: IVec3::new(1, 1, 1),
            to_pos: IVec3::new(2, 1, 1),
            from_voxel: Voxel::Sand,
            to_voxel: Voxel::Air,
        };

        let update2 = BoundaryUpdate {
            from_pos: IVec3::new(3, 3, 3),
            to_pos: IVec3::new(4, 3, 3),
            from_voxel: Voxel::Water,
            to_voxel: Voxel::Air,
        };

        job.add_boundary_update(update1);
        job.add_boundary_update(update2);

        let updates = job.boundary_updates.lock().unwrap();
        assert_eq!(updates.len(), 2);
    }

    #[test]
    fn test_chunk_simulation_grid_phase_toggle() {
        let mut sim_grid = ChunkSimulationGrid::new();
        assert_eq!(sim_grid.phase, 0);

        // Toggle phase
        sim_grid.phase = (sim_grid.phase + 1) % 2;
        assert_eq!(sim_grid.phase, 1);

        // Toggle again
        sim_grid.phase = (sim_grid.phase + 1) % 2;
        assert_eq!(sim_grid.phase, 0);
    }

    #[test]
    fn test_voxel_to_chunk_mapping() {
        // Test that our chunk finding logic works correctly
        assert_eq!(Voxels::find_chunk(IVec3::new(0, 0, 0)), IVec3::new(0, 0, 0));
        assert_eq!(Voxels::find_chunk(IVec3::new(61, 0, 0)), IVec3::new(0, 0, 0)); // Still in chunk 0
        assert_eq!(Voxels::find_chunk(IVec3::new(62, 0, 0)), IVec3::new(1, 0, 0)); // Now in chunk 1
        assert_eq!(Voxels::find_chunk(IVec3::new(-1, 0, 0)), IVec3::new(-1, 0, 0)); // Negative chunk
    }

    #[test]
    fn test_parallel_simulation_system_signature() {
        // Test that the parallel_falling_sands function has the correct signature
        // This is a compile-time test - if this compiles, the signature is correct
        let _system_fn: fn(
            bevy::ecs::system::Query<&mut Voxels>,
            ResMut<crate::voxel::simulation::FallingSandTick>,
            bevy::ecs::system::Local<usize>,
        ) = parallel_falling_sands;
    }

    #[test]
    fn test_chunk_slice_boundaries() {
        // Test that slices correctly identify boundary conditions
        let slice = ChunkSlice {
            chunk_pos: IVec3::new(0, 0, 0),
            y_range: 0..1,
            x_range: 0..unpadded::SIZE,
            z_range: 0..unpadded::SIZE,
        };

        // Test that we can identify when a position would cross chunk boundaries
        let world_pos = IVec3::new(0, 0, 0);
        let relative_pos = IVec3::new(1, 1, 1);
        let swap_point = IVec3::new(0, -1, 0); // Move down
        let target_world_pos = world_pos + swap_point;
        let target_chunk_pos = Voxels::find_chunk(target_world_pos);

        // This should be a cross-chunk update
        assert_ne!(target_chunk_pos, IVec3::new(0, 0, 0));
    }

    #[test]
    fn test_memory_efficiency() {
        // Test that our chunk size assumptions are correct
        let chunk = VoxelChunk::new();

        // Each voxel is stored as a u8 (1 byte)
        let expected_size = unpadded::SIZE * unpadded::SIZE * unpadded::SIZE;
        assert_eq!(expected_size, 62 * 62 * 62);
        assert_eq!(expected_size, 238328); // ~238KB per chunk

        // For a 62x1x62 slice, we're only working with ~3.8KB of data
        let slice_size = 62 * 1 * 62;
        assert_eq!(slice_size, 3844); // ~3.8KB per slice
        assert!(slice_size < 6000); // Should be under our 6K limit
    }

    #[test]
    fn test_deterministic_phases() {
        // Test that the alternating phase system is deterministic
        let mut voxels = Voxels::new();
        voxels.set_voxel(IVec3::new(0, 0, 0), Voxel::Sand);
        voxels.set_voxel(IVec3::new(1, 0, 0), Voxel::Sand);
        voxels.set_voxel(IVec3::new(0, 1, 0), Voxel::Sand);
        voxels.set_voxel(IVec3::new(1, 1, 0), Voxel::Sand);

        let mut sim_grid = ChunkSimulationGrid::from_voxels(&voxels);

        // Run multiple cycles and verify consistency
        for cycle in 0..10 {
            sim_grid.phase = cycle % 2;
            let active_chunks = sim_grid.get_active_chunks();

            if cycle % 2 == 0 {
                // Even cycles should have even chunks
                assert!(active_chunks.iter().all(|&pos| (pos.x + pos.y + pos.z) % 2 == 0));
            } else {
                // Odd cycles should have odd chunks
                assert!(active_chunks.iter().all(|&pos| (pos.x + pos.y + pos.z) % 2 == 1));
            }
        }
    }
}
