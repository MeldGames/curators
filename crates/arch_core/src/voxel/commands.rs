use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;
use serde::{Deserialize, Serialize};

use crate::sdf::{SdfNode, Sdf};
use crate::voxel::data::linearize;
use crate::voxel::simulation::SimChunks;
use crate::voxel::tree::VoxelTree;
use crate::voxel::{Voxel, VoxelNode, VoxelSet, Voxels};
use crate::sdf::voxel_rasterize::{PointIter, ChunkIntersectIter};

pub fn plugin(app: &mut App) {
    app.register_type::<VoxelCommands>();

    app.add_systems(FixedPreUpdate, VoxelCommands::apply_commands);
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

#[derive(Component, Clone, Debug, Reflect, Default)]
#[reflect(Component)]
pub struct VoxelCommands {
    queue: Vec<VoxelCommand>,
}

impl VoxelCommands {
    pub fn apply_commands(mut voxels: Query<(&mut Voxels, &mut SimChunks, &mut VoxelCommands)>) {
        for (mut voxels, mut sim, mut queue) in &mut voxels {
            for command in queue.queue.drain(..) {
                command.apply_sim(&mut *sim);
                command.apply_tree(&mut voxels.tree);
            }
        }
    }

    pub fn push(&mut self, command: VoxelCommand) {
        self.queue.push(command);
    }
}

/// Commands for setting voxels across simulation/tree/network.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
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
        // info!("applying command to sim: {:?}", self);

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

                for sdf_point in PointIter::from_sdf(sdf) {
                    let point = *origin + sdf_point;
                    if sdf.sdf(point.as_vec3()) >= params.within {
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
