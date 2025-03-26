use arch::{ClientPlugin, SharedPlugin};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ClientPlugin)
        .add_plugins(SharedPlugin)
        .run();
}
