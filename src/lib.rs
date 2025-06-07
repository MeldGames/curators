use avian3d::prelude::*;
use bevy::core_pipeline::auto_exposure::AutoExposurePlugin;
use bevy::core_pipeline::experimental::taa::TemporalAntiAliasPlugin;
use bevy::pbr::wireframe::WireframePlugin;
use bevy::prelude::*;
use iyes_perf_ui::prelude::*;

pub mod camera;
pub mod character;
pub mod cursor;
pub mod item;
pub mod ssao;
pub mod voxel;

pub fn server(app: &mut App) {}

pub fn client(app: &mut App) {
    app.add_plugins(camera::plugin).add_plugins(cursor::plugin);
}

pub fn shared(app: &mut App) {
    app.add_plugins(bevy_enhanced_input::EnhancedInputPlugin)
        .add_plugins(PhysicsPlugins::default());
    app.add_plugins(PhysicsDebugPlugin::default());
    app.add_plugins(TemporalAntiAliasPlugin);
    app.add_plugins(ssao::plugin);

    app.add_plugins(voxel::VoxelPlugin::default())
        .add_plugins(item::plugin)
        .add_plugins(WireframePlugin::default())
        .add_plugins(AutoExposurePlugin)
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(bevy::diagnostic::EntityCountDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::SystemInformationDiagnosticsPlugin)
        .add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin)
        .add_plugins(PerfUiPlugin);

    app.world_mut().spawn(PerfUiAllEntries::default());
}
