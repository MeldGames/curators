//! Use inputs to affect the kinematic controller's velocity/position

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use super::input::{Dig, Move, PlayerInput};
use super::kinematic::{KCCGrounded, KinematicCharacterController};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, apply_movement);
}

pub fn apply_movement(
    mut players: Query<(
        &mut KinematicCharacterController,
        &KCCGrounded,
        &mut Transform,
        &Actions<PlayerInput>,
    )>,
    time: Res<Time>,
) -> Result<()> {
    for (mut controller, grounded, mut transform, actions) in &mut players {
        let move_input = actions.value::<Move>()?;
        let dig = actions.value::<Dig>()?;

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

    Ok(())
}
