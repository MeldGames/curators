use bevy::prelude::*;

pub mod grid;
// pub mod naive_mesh;
// pub mod surface_net;

#[derive(Default)]
pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(grid::VoxelGrid::new([50, 50, 50]));
    }
}
