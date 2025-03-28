use bevy::prelude::*;

pub mod voxel;
pub mod camera;

pub struct ServerPlugin;
impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {}
}

pub struct ClientPlugin;
impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(camera::CameraControllerPlugin);
    }
}

pub struct SharedPlugin;
impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(voxel::VoxelPlugin::default());
    }
}
