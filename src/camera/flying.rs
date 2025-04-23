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

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use bevy_enhanced_input::prelude::*;

/// A freecam-style camera controller plugin.
pub fn plugin(app: &mut App) {
    app.add_input_context::<Flying>();
    app.add_systems(Update, run_camera_controller);

    app.add_observer(camera_binding);
}

/// Based on Valorant's default sensitivity, not entirely sure why it is exactly
/// 1.0 / 180.0, but I'm guessing it is a misunderstanding between
/// degrees/radians and then sticking with it because it felt nice.
pub const RADIANS_PER_DOT: f32 = 1.0 / 180.0;

#[derive(InputContext)]
pub struct Flying;

#[derive(InputAction, Debug)]
#[input_action(output = Vec3)]
pub struct CameraMove;

#[derive(InputAction, Debug)]
#[input_action(output = Vec2)]
pub struct CameraRotate;

pub struct SixDOF<I: IntoBindings> {
    pub forward: I,
    pub backward: I,
    pub left: I,
    pub right: I,
    pub up: I,
    pub down: I,
}

impl SixDOF<KeyCode> {
    pub fn wasd_space_ctrl() -> Self {
        SixDOF {
            forward: KeyCode::KeyW,
            backward: KeyCode::KeyS,
            left: KeyCode::KeyA,
            right: KeyCode::KeyD,
            up: KeyCode::Space,
            down: KeyCode::ControlLeft,
        }
    }
}

impl<I: IntoBindings> IntoBindings for SixDOF<I> {
    fn into_bindings(self) -> impl Iterator<Item = InputBinding> {
        // Z
        let forward =
            self.forward.into_bindings().map(|binding| binding.with_modifiers(SwizzleAxis::ZYX));

        // -Z
        let backward = self
            .backward
            .into_bindings()
            .map(|binding| binding.with_modifiers((Negate::all(), SwizzleAxis::ZYX)));

        // X
        let left = self.left.into_bindings().map(|binding| binding);

        // -X
        let right = self.right.into_bindings().map(|binding| binding.with_modifiers(Negate::all()));

        // Y
        let up = self.up.into_bindings().map(|binding| binding.with_modifiers(SwizzleAxis::YXZ));

        // -Y
        let down = self
            .down
            .into_bindings()
            .map(|binding| binding.with_modifiers((Negate::all(), SwizzleAxis::YXZ)));

        forward.chain(backward).chain(left).chain(right).chain(up).chain(down)
    }
}

pub fn camera_binding(trigger: Trigger<Binding<Flying>>, mut flying: Query<&mut Actions<Flying>>) {
    let Ok(mut actions) = flying.get_mut(trigger.entity()) else {
        return;
    };

    actions.bind::<CameraMove>().to(SixDOF {
        forward: KeyCode::KeyW,
        backward: KeyCode::KeyS,
        left: KeyCode::KeyA,
        right: KeyCode::KeyD,
        up: KeyCode::Space,
        down: KeyCode::ControlLeft,
    });

    actions.bind::<CameraRotate>().to(Input::mouse_motion());
}

/// Camera controller [`Component`].
#[derive(Component)]
pub struct CameraController {
    /// Enables this [`CameraController`] when `true`.
    pub enabled: bool,
    /// Indicates if this controller has been initialized by the
    /// [`CameraControllerPlugin`].
    pub initialized: bool,
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
    /// Multiplier for how the mouse scroll wheel modifies
    /// [`walk_speed`](CameraController::walk_speed)
    /// and [`run_speed`](CameraController::run_speed).
    pub scroll_factor: f32,
    /// Friction factor used to exponentially decay
    /// [`velocity`](CameraController::velocity) over time.
    pub friction: f32,
    /// This [`CameraController`]'s pitch rotation.
    pub pitch: f32,
    /// This [`CameraController`]'s yaw rotation.
    pub yaw: f32,
    /// This [`CameraController`]'s translation velocity.
    pub velocity: Vec3,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            enabled: true,
            initialized: false,
            sensitivity: 1.0,
            key_run: KeyCode::ShiftLeft,
            mouse_key_cursor_grab: MouseButton::Right,
            keyboard_key_toggle_cursor_grab: KeyCode::KeyM,
            walk_speed: 8.0,
            run_speed: 15.0,
            scroll_factor: 1.0,
            friction: 0.5,
            pitch: 0.0,
            yaw: 0.0,
            velocity: Vec3::ZERO,
        }
    }
}

impl fmt::Display for CameraController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "
Freecam Controls:
    Mouse\t- Move camera orientation
    Scroll\t- Adjust movement speed
    {:?}\t- Hold to grab cursor
    {:?}\t- Toggle cursor grab
    {:?}\t- Fly faster while held",
            self.mouse_key_cursor_grab, self.keyboard_key_toggle_cursor_grab, self.key_run,
        )
    }
}

fn run_camera_controller(
    time: Res<Time>,
    mut windows: Query<&mut Window>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    accumulated_mouse_scroll: Res<AccumulatedMouseScroll>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut toggle_cursor_grab: Local<bool>,
    mut mouse_cursor_grab: Local<bool>,
    mut query: Query<(&mut Transform, &mut CameraController), With<Camera>>,
) {
    let dt = time.delta_secs();

    let (mut transform, mut controller) = query.single_mut();

    if !controller.initialized {
        let (yaw, pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);
        controller.yaw = yaw;
        controller.pitch = pitch;
        controller.initialized = true;
        info!("{}", *controller);
    }
    if !controller.enabled {
        return;
    }

    // Handle key input
    let mut axis_input = Vec3::ZERO;

    let mut cursor_grab_change = false;
    if key_input.just_pressed(controller.keyboard_key_toggle_cursor_grab) {
        *toggle_cursor_grab = !*toggle_cursor_grab;
        cursor_grab_change = true;
    }
    if mouse_button_input.just_pressed(controller.mouse_key_cursor_grab) {
        *mouse_cursor_grab = true;
        cursor_grab_change = true;
    }
    if mouse_button_input.just_released(controller.mouse_key_cursor_grab) {
        *mouse_cursor_grab = false;
        cursor_grab_change = true;
    }
    let cursor_grab = *mouse_cursor_grab || *toggle_cursor_grab;

    // Apply movement update
    if axis_input != Vec3::ZERO {
        let max_speed = if key_input.pressed(controller.key_run) {
            controller.run_speed
        } else {
            controller.walk_speed
        };
        controller.velocity = axis_input.normalize() * max_speed;
    } else {
        let friction = controller.friction.clamp(0.0, 1.0);
        controller.velocity *= 1.0 - friction;
        if controller.velocity.length_squared() < 1e-6 {
            controller.velocity = Vec3::ZERO;
        }
    }
    let forward = *transform.forward();
    let right = *transform.right();
    transform.translation += controller.velocity.x * dt * right
        + controller.velocity.y * dt * Vec3::Y
        + controller.velocity.z * dt * forward;

    // Handle cursor grab
    if cursor_grab_change {
        if cursor_grab {
            for mut window in &mut windows {
                if !window.focused {
                    continue;
                }

                window.cursor_options.grab_mode = CursorGrabMode::Locked;
                window.cursor_options.visible = false;
            }
        } else {
            for mut window in &mut windows {
                window.cursor_options.grab_mode = CursorGrabMode::None;
                window.cursor_options.visible = true;
            }
        }
    }

    // Handle mouse input
    if accumulated_mouse_motion.delta != Vec2::ZERO && cursor_grab {
        // Apply look update
        controller.pitch = (controller.pitch
            - accumulated_mouse_motion.delta.y * RADIANS_PER_DOT * controller.sensitivity)
            .clamp(-PI / 2., PI / 2.);
        controller.yaw -=
            accumulated_mouse_motion.delta.x * RADIANS_PER_DOT * controller.sensitivity;
        transform.rotation = Quat::from_euler(EulerRot::ZYX, 0.0, controller.yaw, controller.pitch);
    }
}
