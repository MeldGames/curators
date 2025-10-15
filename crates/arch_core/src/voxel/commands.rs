use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;
use serde::{Deserialize, Serialize};

use crate::sdf::{SdfNode, Sdf};
use crate::voxel::simulation::SimChunks;
use crate::voxel::tree::VoxelTree;
use crate::voxel::{Voxel, VoxelSet, Voxels};
use crate::sdf::voxel_rasterize::{RasterConfig, rasterize};

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
    SetVoxelsSdf { center: IVec3, sdf: SdfNode, voxel: Voxel, params: SetVoxelsSdfParams },
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
            Self::SetVoxelsSdf { center, sdf, voxel, params } => {
                // TODO: Get the overlapping chunks and the overlaps in the chunks for setting.
                // This should save us a lot of lookup time for setting.
                let aabb = sdf.aabb().expect("Can't set voxels SDF without a bound");

                let raster_config = RasterConfig {
                    clip_bounds: Aabb3d { min: Vec3A::splat(-1000.0), max: Vec3A::splat(1000.0) },
                    grid_scale: crate::voxel::GRID_SCALE,
                    pad_bounds: Vec3::splat(0.0),
                };

                for raster in rasterize(sdf, raster_config) {
                    let point = *center + raster.point;
                    if raster.distance >= params.within {
                        continue;
                    }

                    let current_voxel = tree.get_voxel(point);
                    if params.can_replace.contains(current_voxel) {
                        set += 1;
                        tree.set_voxel(point, *voxel);
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
            Self::SetVoxelsSdf { center, sdf, voxel, params } => {
                // TODO: Get the overlapping chunks and the overlaps in the chunks for setting.
                // This should save us a lot of lookup time for setting.

                use crate::sdf::voxel_rasterize::{RasterConfig, rasterize};
                let raster_config = RasterConfig {
                    clip_bounds: Aabb3d { min: Vec3A::splat(-1000.0), max: Vec3A::splat(1000.0) },
                    grid_scale: crate::voxel::GRID_SCALE,
                    pad_bounds: Vec3::splat(0.0),
                };

                for raster in rasterize(sdf, raster_config) {
                    let point = *center + raster.point;
                    if raster.distance >= params.within {
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
