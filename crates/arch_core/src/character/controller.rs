//! Use inputs to affect the kinematic controller's velocity/position

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::character::input::Jump;
use crate::character::kinematic::KCCJump;

use super::input::{Dig, Move, PlayerInput};
use super::kinematic::{KCCGrounded, KinematicCharacterController};

pub(super) fn plugin(app: &mut App) {
    app.add_observer(apply_movement);
}

pub fn apply_movement(
    trigger: Trigger<Fired<Move>>,
    mut players: Query<(
        &mut KinematicCharacterController,
        &KCCGrounded,
        &mut Transform,
        &Actions<PlayerInput>,
    )>,
    time: Res<Time>,
) {
    let Ok((mut controller, _grounded, mut transform, actions)) = players.get_mut(trigger.target())
    else {
        return;
    };

    let move_input = trigger.value;
    // let dig = actions.value::<Dig>()?; // how do i get this value now?
    let dig = false;

    let speed = 5.0;

    // TODO: Smooth out the change from normal speed to digging speed.
    // Maybe slow more the further you are from the current target block?
    let dig_max_speed = 2.0; // if digging this is the max speed.
    let mut movement = move_input.normalize_or_zero() * speed;
    if dig {
        if movement.x > dig_max_speed {
            movement.x = dig_max_speed;
        }
        if movement.x < -dig_max_speed {
            movement.x = -dig_max_speed;
        }

        if movement.y > dig_max_speed {
            movement.y = dig_max_speed;
        }
        if movement.y < -dig_max_speed {
            movement.y = -dig_max_speed;
        }
    }

    if movement.x != 0.0 {
        controller.velocity.x = movement.x;
    }

    if movement.y != 0.0 {
        controller.velocity.z = -movement.y;
    }

    if controller.velocity.y.is_nan() {
        controller.velocity.y = 0.0;
    }

    if movement.length_squared() > 0.0 {
        let angle = movement.y.atan2(movement.x) - std::f32::consts::PI / 2.0;
        let target = Quat::from_axis_angle(Vec3::Y, angle);
        transform.rotation = transform.rotation.slerp(target, time.delta_secs() * 20.0);
    }
}

pub fn apply_jump(
    trigger: Trigger<Fired<Jump>>,
    mut players: Query<(
        &mut KinematicCharacterController,
        &KCCGrounded,
        &mut KCCJump,
        &Actions<PlayerInput>,
    )>,
    time: Res<Time>,
)  {
    let Ok((mut controller, grounded, mut jump, actions)) = players.get_mut(trigger.target()) else {
        return;
    };

    let mut falloff = 0.0;
    match trigger.state {
        ActionState::Fired => {
            if grounded.grounded {
                if jump.current_force.is_none() && !jump.last_jump {
                    jump.last_jump = true;
                    jump.current_force = Some(jump.initial_force);
                } else if jump.current_force.is_some() {
                    jump.current_force = None;
                }
            } else {
                falloff = jump.hold_falloff;
            }
        },
        _ => {
            jump.last_jump = false;
            falloff = jump.falloff;
        },
    }

    if let Some(force) = &mut jump.current_force {
        *force -= falloff * time.delta_secs();
    }

    if jump.current_force.is_some_and(|force| force < 0.0) {
        jump.current_force = None;
    }

    if let Some(force) = jump.current_force {
        controller.velocity += Vec3::Y * force;
    }
}
