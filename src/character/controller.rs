//! Use inputs to affect the kinematic controller's velocity/position

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use super::{input::{PlayerInput, Move}, kinematic::KinematicCharacterController};


pub(super) fn plugin(app: &mut App) {
    app.add_observer(apply_movement);
}

pub fn apply_movement(trigger: Trigger<Fired<Move>>, mut players: Query<&mut KinematicCharacterController>) {
    let mut controller = players.get_mut(trigger.entity()).unwrap();
    let speed = 5.0;
    controller.velocity.x = trigger.value.x * speed;
    controller.velocity.z = -trigger.value.y * speed;

    if controller.velocity.y.is_nan() {
        controller.velocity.y = 0.0;
    }
}