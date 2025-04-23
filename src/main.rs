use arch::{ClientPlugin, SharedPlugin};
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(SharedPlugin)
        .add_plugins(ClientPlugin)
        .run();
}
