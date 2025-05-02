//! Use inputs to affect the kinematic controller's velocity/position

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use super::input::{Jump, Move, PlayerInput};
use super::kinematic::{KCCGravity, KCCGrounded, KinematicCharacterController};

pub(super) fn plugin(app: &mut App) {
    app.add_observer(apply_movement);
}

pub fn apply_movement(
    trigger: Trigger<Fired<Move>>,
    mut players: Query<(&mut KinematicCharacterController, &KCCGrounded)>,
) {
    let (mut controller, grounded) = players.get_mut(trigger.entity()).unwrap();
    /*if !grounded.grounded {
        return;
    }*/

    let speed = 5.0;
    if trigger.value.x != 0.0 {
        controller.velocity.x = trigger.value.x * speed;
    }

    if trigger.value.y != 0.0 {
        controller.velocity.z = -trigger.value.y * speed;
    }

    if controller.velocity.y.is_nan() {
        controller.velocity.y = 0.0;
    }
}