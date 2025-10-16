//! A freecam-style camera controller plugin.
//! To use in your own application:
//! - Copy the code for the [`CameraControllerPlugin`] and add the plugin to
//!   your App.
//! - Attach the [`CameraController`] component to an entity with a
//!   [`Camera3d`].
//!
//! Unlike other examples, which demonstrate an application, this demonstrates a
//! plugin library.

use std::f32::consts::*;
use std::fmt;

use bevy::prelude::*;
use bevy::window::CursorOptions;
use bevy_enhanced_input::prelude::*;

use crate::cursor::CursorGrabOffset;

/// A freecam-style camera controller plugin.
pub fn plugin(app: &mut App) {
    app.add_input_context::<FlyingCamera>();

    app.add_observer(started_flying).add_observer(handle_movement).add_observer(handle_rotation);
}

/// Based on Valorant's default sensitivity, not entirely sure why it is exactly
/// 1.0 / 180.0, but I'm guessing it is a misunderstanding between
/// degrees/radians and then sticking with it because it felt nice.
pub const RADIANS_PER_DOT: f32 = 1.0 / 180.0;

#[derive(Component)]
// #[input_context(priority = 10)]
pub struct FlyingCamera;

#[derive(InputAction, Debug)]
#[action_output(Vec3)]
pub struct CameraMove;

#[derive(InputAction, Debug)]
#[action_output(Vec2)]
pub struct CameraRotate;

/// Camera controller [`Component`].
#[derive(Component)]
pub struct FlyingSettings {
    /// Multiplier for pitch and yaw rotation speed.
    pub sensitivity: f32,
    /// [`KeyCode`] to use [`run_speed`](CameraController::run_speed) instead of
    /// [`walk_speed`](CameraController::walk_speed) for translation.
    pub key_run: KeyCode,
    /// [`MouseButton`] for grabbing the mouse focus.
    pub mouse_key_cursor_grab: MouseButton,
    /// [`KeyCode`] for grabbing the keyboard focus.
    pub keyboard_key_toggle_cursor_grab: KeyCode,
    /// Multiplier for unmodified translation speed.
    pub walk_speed: f32,
    /// Multiplier for running translation speed.
    pub run_speed: f32,
    /// Friction factor used to exponentially decay
    /// [`velocity`](CameraController::velocity) over time.
    pub friction: f32,
}

#[derive(Component)]
pub struct FlyingState {
    /// This [`CameraController`]'s pitch rotation.
    pub pitch: f32,
    /// This [`CameraController`]'s yaw rotation.
    pub yaw: f32,
    /// This [`CameraController`]'s translation velocity.
    pub velocity: Vec3,
}

impl Default for FlyingSettings {
    fn default() -> Self {
        Self {
            sensitivity: 1.0,
            key_run: KeyCode::ShiftLeft,
            mouse_key_cursor_grab: MouseButton::Right,
            keyboard_key_toggle_cursor_grab: KeyCode::KeyM,
            walk_speed: 8.0,
            run_speed: 15.0,
            friction: 0.5,
        }
    }
}

impl Default for FlyingState {
    fn default() -> Self {
        Self { pitch: 0.0, yaw: 0.0, velocity: Vec3::ZERO }
    }
}

impl fmt::Display for FlyingSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "
Freecam Controls:
    Mouse\t- Move camera orientation
    {:?}\t- Hold to grab cursor
    {:?}\t- Toggle cursor grab
    {:?}\t- Fly faster while held",
            self.mouse_key_cursor_grab, self.keyboard_key_toggle_cursor_grab, self.key_run,
        )
    }
}

pub fn started_flying(
    trigger: Trigger<OnInsert, ContextActivity<FlyingCamera>>,
    mut query: Query<(&Transform, &ContextActivity<FlyingCamera>, &mut FlyingState)>,
) {
    let Ok((transform, context_activity, mut state)) = query.get_mut(trigger.target()) else {
        return;
    };

    if !**context_activity {
        return;
    }

    let (yaw, pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);
    state.yaw = yaw;
    state.pitch = pitch;
}

pub fn handle_rotation(
    trigger: Trigger<Fired<CameraRotate>>,
    mut camera: Query<(&mut Transform, &mut FlyingState, &FlyingSettings)>,
    windows: Query<(&Window, &CursorOptions)>,
    mut cursor_grab_offset: ResMut<CursorGrabOffset>,
) -> Result<()> {
    let Ok((mut transform, mut state, settings)) = camera.get_mut(trigger.target()) else {
        return Ok(());
    };

    let mut rotation = trigger.value;

    // If the cursor grab setting caused this, prevent it from doing anything.
    if !crate::cursor::cursor_grabbed(windows) {
        return Ok(());
    }

    // Handle mouse input
    if rotation != Vec2::ZERO {
        if cursor_grab_offset.is_none() {
            // Unknown delta, ignore this one.
            cursor_grab_offset.0 = Some(Vec2::ZERO);
            return Ok(());
        }

        rotation += cursor_grab_offset.unwrap();
        cursor_grab_offset.0 = Some(Vec2::ZERO);

        // Apply look update
        state.pitch = (state.pitch - rotation.y * RADIANS_PER_DOT * settings.sensitivity)
            .clamp(-PI / 2., PI / 2.);
        state.yaw -= rotation.x * RADIANS_PER_DOT * settings.sensitivity;
        transform.rotation = Quat::from_euler(EulerRot::ZYX, 0.0, state.yaw, state.pitch);
    }

    Ok(())
}

pub fn handle_movement(
    trigger: Trigger<Fired<CameraMove>>,
    time: Res<Time>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &FlyingSettings, &mut FlyingState), With<Camera>>,
) {
    let dt = time.delta_secs();

    let Ok((mut transform, settings, mut state)) = query.get_mut(trigger.target()) else {
        return;
    };
    let movement = trigger.value;

    // Apply movement update
    if movement != Vec3::ZERO {
        let max_speed = if key_input.pressed(settings.key_run) {
            settings.run_speed
        } else {
            settings.walk_speed
        };
        state.velocity = movement.normalize() * max_speed;
    } else {
        let friction = settings.friction.clamp(0.0, 1.0);
        state.velocity *= 1.0 - friction;
        if state.velocity.length_squared() < 1e-6 {
            state.velocity = Vec3::ZERO;
        }
    }
    let forward = *transform.forward();
    let right = *transform.right();
    // let up = Vec3::Y;
    let up = *transform.up();
    transform.translation += state.velocity.x * dt * right
        + state.velocity.y * dt * up
        + -state.velocity.z * dt * forward;
}
