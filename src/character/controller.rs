use avian3d::prelude::*;
use bevy::prelude::*;

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(());
    }
}
