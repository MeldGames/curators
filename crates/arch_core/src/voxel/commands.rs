use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::sdf::{SdfNode, Sdf};
use crate::voxel::data::linearize;
use crate::voxel::simulation::SimChunks;
use crate::voxel::tree::VoxelTree;
use crate::voxel::{SimStep, Voxel, VoxelNode, VoxelSet, Voxels};
use crate::sdf::voxel_rasterize::{PointIter, ChunkIntersectIter};

pub fn plugin(app: &mut App) {
    app.register_type::<VoxelCommand>();
    app.add_event::<VoxelCommand>();

    app.add_systems(FixedPostUpdate, apply_sim.in_set(SimStep::AddVoxelsToSim));
    app.add_systems(PostUpdate, apply_tree);
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct SetVoxelParams {
    pub can_replace: VoxelSet,
}

impl Default for SetVoxelParams {
    fn default() -> Self {
        Self { can_replace: VoxelSet::from_voxel(Voxel::Air) }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct SetVoxelsSdfParams {
    pub within: f32,
    pub can_replace: VoxelSet,
}

impl Default for SetVoxelsSdfParams {
    fn default() -> Self {
        Self { within: 0.0, can_replace: VoxelSet::from_voxel(Voxel::Air) }
    }
}

pub fn apply_tree(mut voxels: Query<&mut Voxels>, mut commands: EventReader<VoxelCommand>) {
    for command in commands.read() {
        for mut voxels in &mut voxels {
            command.apply_tree(&mut voxels.tree);
        }
    }
}

pub fn apply_sim(mut sims: Query<&mut SimChunks>, mut commands: EventReader<VoxelCommand>) {
    for command in commands.read() {
        for mut sim in &mut sims {
            command.apply_sim(&mut *sim);
        }
    }
}

/// Commands for setting voxels across simulation/tree/network.
#[derive(Event, Debug, Clone, Serialize, Deserialize, Reflect)]
pub enum VoxelCommand {
    SetVoxel { point: IVec3, voxel: Voxel, params: SetVoxelParams },
    SetVoxelsSdf { origin: IVec3, sdf: SdfNode, voxel: Voxel, params: SetVoxelsSdfParams },
}

impl VoxelCommand {
    pub fn apply_tree(&self, tree: &mut VoxelTree) {
        // info!("applying command to tree: {:?}", self);

        let mut set = 0;
        match self {
            Self::SetVoxel { point, voxel, params } => {
                let current_voxel = tree.get_voxel(*point);
                if params.can_replace.contains(current_voxel) {
                    set += 1;
                    tree.set_voxel(*point, *voxel);
                }
            },
            Self::SetVoxelsSdf { origin, sdf, voxel, params } => {
                let sdf = sdf.translate(origin.as_vec3());
                let intersections = ChunkIntersectIter::from_sdf(sdf.clone(), 16);
                for (chunk_point, local_points) in intersections {
                    if !tree.chunk_point_in_bounds(*chunk_point) {
                        continue;
                    }

                    let chunk = tree.get_chunk_mut(*chunk_point);
                    chunk.subdivide();
                    let VoxelNode::Leaf { leaf, ..} = chunk else {
                        error!("chunk was not a leaf");
                        continue;
                    };

                    let chunk_min = chunk_point.0 * IVec3::splat(16);
                    for local_point in local_points {
                        let world_point = chunk_min + local_point;
                        let distance = sdf.sdf(world_point.as_vec3());

                        if distance >= params.within {
                            continue;
                        }

                        let index = linearize(local_point);
                        let current_voxel = leaf[index];
                        if params.can_replace.contains(current_voxel) {
                            set += 1;
                            leaf[index] = *voxel;
                        }
                    }
                }
            },
        }

        // info!("{} voxels set from command", set);
    }

    pub fn apply_sim(&self, sim_chunks: &mut SimChunks) {
        info!("applying command to sim: {:?}", self);

        let mut set = 0;
        match self {
            Self::SetVoxel { point, voxel, params } => {
                if let Some(current_voxel) = sim_chunks.get_voxel(*point) {
                    if params.can_replace.contains(current_voxel) {
                        set += 1;
                        sim_chunks.set_voxel(*point, *voxel);
                    }
                }
            },
            Self::SetVoxelsSdf { origin, sdf, voxel, params } => {
                // TODO: Get the overlapping chunks and the overlaps in the chunks for setting.
                // This should save us a lot of lookup time for setting.
                let sdf = sdf.translate(origin.as_vec3());
                for point in PointIter::from_sdf(&sdf) {
                    let dist = sdf.sdf(point.as_vec3());
                    if dist >= params.within {
                        continue;
                    }

                    if let Some(current_voxel) = sim_chunks.get_voxel(point) {
                        if params.can_replace.contains(current_voxel) {
                            set += 1;
                            sim_chunks.set_voxel(point, *voxel);
                        }
                    }
                }
            },
        }

        // info!("{} voxels set in sim from command", set);
    }
}
