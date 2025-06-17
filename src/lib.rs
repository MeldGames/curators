use avian3d::prelude::*;
use bevy::core_pipeline::auto_exposure::AutoExposurePlugin;
use bevy::core_pipeline::core_3d::graph::Node3d;
use bevy::core_pipeline::experimental::taa::TemporalAntiAliasPlugin;
use bevy::pbr::wireframe::WireframePlugin;
use bevy::prelude::*;
use bevy_edge_detection::*;
use bevy_mod_outline::*;
use iyes_perf_ui::prelude::*;

pub mod camera;
pub mod character;
pub mod cursor;
pub mod map;
pub mod item;
pub mod ssao;
pub mod tool;
pub mod voxel;

pub fn server(app: &mut App) {}

pub fn client(app: &mut App) {
    app.add_plugins(camera::plugin).add_plugins(cursor::plugin);
}

pub fn shared(app: &mut App) {
    app.add_plugins(bevy_enhanced_input::EnhancedInputPlugin)
        .add_plugins(PhysicsPlugins::default());
    // app.add_plugins(PhysicsDebugPlugin::default());
    app.add_plugins(TemporalAntiAliasPlugin);
    app.add_plugins(ssao::plugin);

    app.add_plugins((OutlinePlugin, AutoGenerateOutlineNormalsPlugin::default()));

    app.add_plugins(voxel::VoxelPlugin::default())
        .add_plugins(item::plugin)
        .add_plugins(map::plugin)
        .add_plugins(EdgeDetectionPlugin {
            // If you wish to apply Smaa anti-aliasing after edge detection,
            // please ensure that the rendering order of [`EdgeDetectionNode`] is set before
            // [`SmaaNode`].
            before: Node3d::Smaa,
        })
        .add_plugins(WireframePlugin::default())
        .add_plugins(AutoExposurePlugin)
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(bevy::diagnostic::EntityCountDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::SystemInformationDiagnosticsPlugin)
        .add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin)
        .add_plugins(PerfUiPlugin);

    app.world_mut().spawn(PerfUiAllEntries::default());
}
