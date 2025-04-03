use bevy::{pbr::wireframe::WireframePlugin, prelude::*};

pub mod camera;
pub mod character;
pub mod voxel;

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
        app.add_plugins(voxel::VoxelPlugin::default()).add_plugins(WireframePlugin::default());
    }
}
