use std::f32::consts::*;
use std::fmt;

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::cursor::CursorGrabOffset;

/// A freecam-style camera controller plugin.
pub fn plugin(app: &mut App) {
    app.add_input_context::<FlyingCamera>();
    app.add_systems(Update, (handle_movement, handle_rotation));

    app.add_observer(camera_binding).add_observer(started_flying);
}

/// Based on Valorant's default sensitivity, not entirely sure why it is exactly
/// 1.0 / 180.0, but I'm guessing it is a misunderstanding between
/// degrees/radians and then sticking with it because it felt nice.
pub const RADIANS_PER_DOT: f32 = 1.0 / 180.0;

#[derive(InputContext)]
#[input_context(priority = 10)]
pub struct FirstPersonCamera;

#[derive(InputAction, Debug)]
#[input_action(output = Vec2)]
pub struct CameraRotate;

pub fn camera_binding(
    trigger: Trigger<Bind<FlyingCamera>>,
    mut flying: Query<&mut Actions<FlyingCamera>>,
) {
    let Ok(mut actions) = flying.get_mut(trigger.target()) else {
        return;
    };

    info!("binding flying camera");
    actions.bind::<CameraMove>().to(SixDOF {
        forward: KeyCode::KeyW,
        left: KeyCode::KeyA,
        backward: KeyCode::KeyS,
        right: KeyCode::KeyD,
        up: KeyCode::Space,
        down: KeyCode::ControlRight,
    });

    actions.bind::<CameraRotate>().to(Input::mouse_motion());
}

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
    trigger: Trigger<OnInsert, Actions<FlyingCamera>>, /* Maybe using Binding? Idk doesn't
                                                        * matter much */
    mut query: Query<(&Transform, &mut FlyingState)>,
) {
    let Ok((transform, mut state)) = query.get_mut(trigger.target()) else {
        return;
    };
    let (yaw, pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);
    state.yaw = yaw;
    state.pitch = pitch;
    // info!("{}", *controller);
}

pub fn handle_rotation(
    mut camera: Query<(&mut Transform, &mut FlyingState, &FlyingSettings, &Actions<FlyingCamera>)>,
    windows: Query<&Window>,
    mut cursor_grab_offset: ResMut<CursorGrabOffset>,
) -> Result<()> {
    let Ok((mut transform, mut state, settings, actions)) = camera.single_mut() else {
        return Ok(());
    };

    // If the cursor grab setting caused this, prevent it from doing anything.
    let mut rotation = actions.value::<CameraRotate>()?;

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
    time: Res<Time>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<
        (&mut Transform, &Actions<FlyingCamera>, &FlyingSettings, &mut FlyingState),
        With<Camera>,
    >,
) -> Result<()> {
    let dt = time.delta_secs();

    let Ok((mut transform, actions, settings, mut state)) = query.single_mut() else {
        return Ok(());
    };
    let movement = actions.value::<CameraMove>()?;

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

    // let mut cursor_grab_change = false;
    // if key_input.just_pressed(controller.keyboard_key_toggle_cursor_grab) {
    // toggle_cursor_grab = !*toggle_cursor_grab;
    // cursor_grab_change = true;
    // }
    // if mouse_button_input.just_pressed(controller.mouse_key_cursor_grab) {
    // mouse_cursor_grab = true;
    // cursor_grab_change = true;
    // }
    // if mouse_button_input.just_released(controller.mouse_key_cursor_grab) {
    // mouse_cursor_grab = false;
    // cursor_grab_change = true;
    // }
    // let cursor_grab = *mouse_cursor_grab || *toggle_cursor_grab;
    //
    //
    // Handle cursor grab
    // if cursor_grab_change {
    // if cursor_grab {
    // for mut window in &mut windows {
    // if !window.focused {
    // continue;
    // }
    //
    // window.cursor_options.grab_mode = CursorGrabMode::Locked;
    // window.cursor_options.visible = false;
    // }
    // } else {
    // for mut window in &mut windows {
    // window.cursor_options.grab_mode = CursorGrabMode::None;
    // window.cursor_options.visible = true;
    // }
    // }
    // }

    Ok(())
}
