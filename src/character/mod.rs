
use bevy::prelude::*;


pub mod controller;
pub mod player;
pub mod kinematic;


pub(super) fn plugin(app: &mut App) {
    app
        .add_plugins(kinematic::plugin)
        .add_plugins(player::plugin);

        //.add_plugins(PlayerPlugin)
        //.add_plugins(kinematic::KinematicCharacterController);
}