use bevy::{pbr::wireframe::WireframePlugin, prelude::*};
use iyes_perf_ui::prelude::*;

pub mod camera;
pub mod cursor;
pub mod character;
pub mod voxel;

pub struct ServerPlugin;
impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {}
}

pub struct ClientPlugin;
impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(camera::plugin)
            .add_plugins(cursor::plugin);
    }
}

pub struct SharedPlugin;
impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_enhanced_input::EnhancedInputPlugin);

        app.add_plugins(voxel::VoxelPlugin::default())
            .add_plugins(WireframePlugin::default())
            .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
            .add_plugins(bevy::diagnostic::EntityCountDiagnosticsPlugin)
            .add_plugins(bevy::diagnostic::SystemInformationDiagnosticsPlugin)
            .add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin)
            .add_plugins(PerfUiPlugin);

        app.world_mut().spawn(PerfUiAllEntries::default());
    }
}
