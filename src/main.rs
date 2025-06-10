use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

fn main() -> AppExit {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin { enable_multipass_for_primary_context: true })
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(arch::shared)
        .add_plugins(arch::client)
        .run()
}
