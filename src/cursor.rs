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

pub fn plugin(app: &mut App) {
    app.add_input_context::<Cursor>();
    app.insert_resource(CursorGrabOffset(None));

    app.add_observer(cursor_binding);
    app.add_systems(First, cursor_grab);
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
    let Ok(mut actions) = cursor.get_mut(trigger.entity()) else {
        return;
    };

    actions.bind::<FreeCursor>().to(KeyCode::AltLeft);
}

#[derive(Resource, Deref)]
pub struct CursorGrabOffset(pub Option<Vec2>);

pub fn cursor_grab(
    mut offset: ResMut<CursorGrabOffset>,
    mut windows: Query<&mut Window>,
    cursor: Query<&Actions<Cursor>>,
    grabbers: Query<(), Or<(With<Actions<FlyingCamera>>, With<Actions<PlayerInput>>)>>,
) {
    let cursor = cursor.single();
    let grabbers = grabbers.iter().count();

    for mut window in &mut windows {
        let mut grab = false;
        if window.focused {
            if grabbers > 0 {
                grab = true;
            }

            if cursor.action::<FreeCursor>().value().as_bool() {
                grab = false;
            }
        }

        if grab && window.cursor_options.grab_mode == CursorGrabMode::None {
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
            window.cursor_options.visible = false;

            let center = window.resolution.size() / 2.0;
            offset.0 = window.cursor_position().map(|current| current - center);
            window.set_cursor_position(Some(center));
        } else if !grab && window.cursor_options.grab_mode == CursorGrabMode::Locked {
            window.cursor_options.grab_mode = CursorGrabMode::None;
            window.cursor_options.visible = true;
        }
    }
}
