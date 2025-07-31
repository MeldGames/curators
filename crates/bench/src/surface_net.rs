use arch_core::bevy;
use arch_core::bevy::math::bounding::Aabb3d;
use arch_core::sdf::{self, Sdf};
use arch_core::voxel::{self, GRID_SCALE, Voxel, Voxels};
use bevy::prelude::*;

pub fn plugin_setup() -> App {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugins(voxel::voxels::plugin)
        .insert_resource(voxel::simulation::FallingSandTick(0))
        .add_systems(Update, voxel::simulation::falling_sands)
        .add_plugins(voxel::simulation::data::plugin);
    app
}
