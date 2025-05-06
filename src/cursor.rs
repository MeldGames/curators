use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use bevy_enhanced_input::prelude::*;

use crate::camera::flying::FlyingCamera;
use crate::character::input::PlayerInput;

#[derive(InputContext, Debug)]
#[input_context(priority = 10)]
pub struct Cursor;

#[derive(InputAction, Debug)]
#[input_action(output = bool)]
pub struct FreeCursor;

#[derive(InputAction, Debug)]
#[input_action(output = bool)]
pub struct ToggleCursor;

pub fn plugin(app: &mut App) {
    app.add_input_context::<Cursor>();
    app.insert_resource(CursorGrabOffset(None));
    app.insert_resource(CursorGrabToggle(false));

    app.add_observer(cursor_binding);
    app.add_systems(PostUpdate, cursor_grab);
    app.add_systems(Startup, spawn_cursor_input);
}

// Run condition helper.
pub fn cursor_grabbed(windows: Query<&Window>) -> bool {
    windows.iter().any(|window| window.cursor_options.grab_mode == CursorGrabMode::Locked)
}

pub fn spawn_cursor_input(mut commands: Commands) {
    commands.spawn((Name::new("Cursor Input"), Actions::<Cursor>::default()));
}

pub fn cursor_binding(trigger: Trigger<Binding<Cursor>>, mut cursor: Query<&mut Actions<Cursor>>) {
    let Ok(mut actions) = cursor.get_mut(trigger.target()) else {
        return;
    };

    actions.bind::<FreeCursor>().to(KeyCode::AltLeft);
    actions.bind::<ToggleCursor>().to(KeyCode::KeyZ).with_conditions(JustPress::default());
}

#[derive(Resource, Deref)]
pub struct CursorGrabOffset(pub Option<Vec2>);

#[derive(Resource, Deref, DerefMut)]
pub struct CursorGrabToggle(pub bool);

pub fn cursor_grab(
    mut offset: ResMut<CursorGrabOffset>,
    mut windows: Query<&mut Window>,
    cursor: Query<&Actions<Cursor>>,
    grabbers: Query<(), Or<(With<Actions<FlyingCamera>>, With<Actions<PlayerInput>>)>>,
    mut toggle: ResMut<CursorGrabToggle>,
) {
    let cursor = cursor.single().unwrap();
    let grabbers = grabbers.iter().count();

    if cursor.action::<ToggleCursor>().state() == ActionState::Fired {
        toggle.0 = !toggle.0;
        info!("toggled cursor: {:?}", toggle.0);
    }

    for mut window in &mut windows {
        let mut grab = false;
        if window.focused {
            // Is there anything that wants control of the cursor?
            if grabbers > 0 {
                grab = true;
            }

            if cursor.action::<FreeCursor>().value().as_bool() {
                grab = false;
            }
        }

        // Are we inside the window?
        let position = window.cursor_position();
        if position.is_none() {
            grab = false;
        }

        // Is the cursor grab toggled off? Then don't grab.
        if !toggle.0 {
            grab = false;
        }

        if grab && window.cursor_options.grab_mode == CursorGrabMode::None {
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
            window.cursor_options.visible = false;

            let center = window.resolution.size() / 2.0;
            offset.0 = window.cursor_position().map(|current| current - center);
            window.set_cursor_position(Some(center)); // TODO: Figure out a way to ignore this for camera movement.
        } else if !grab && window.cursor_options.grab_mode == CursorGrabMode::Locked {
            window.cursor_options.grab_mode = CursorGrabMode::None;
            window.cursor_options.visible = true;
        }
    }
}
