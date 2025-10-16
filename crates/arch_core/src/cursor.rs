use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use bevy_enhanced_input::prelude::{*, Press};

use crate::camera::flying::FlyingCamera;
use crate::character::input::PlayerInput;

#[derive(Component, Debug)]
pub struct Cursor;

#[derive(InputAction, Debug)]
#[action_output(bool)]
pub struct FreeCursor;

#[derive(InputAction, Debug)]
#[action_output(bool)]
pub struct ToggleCursor;

pub fn plugin(app: &mut App) {
    app.add_input_context::<Cursor>();
    app.insert_resource(CursorGrabOffset(None));
    app.insert_resource(CursorGrabToggle(false));

    app.add_systems(PostUpdate, cursor_grab);
    app.add_systems(Startup, spawn_cursor_input);
}

// Run condition helper.
pub fn cursor_grabbed(windows: Query<(&Window, &CursorOptions)>) -> bool {
    windows.iter().any(|(window, cursor_options)| cursor_options.grab_mode == CursorGrabMode::Locked)
}

pub fn spawn_cursor_input(mut commands: Commands) {
    commands.spawn((
        Name::new("Cursor Input"),
        Cursor,
        actions!(Cursor[
            (
                Action::<FreeCursor>::new(),
                bindings![KeyCode::AltLeft],
            ),
            (
                Action::<ToggleCursor>::new(),
                Press::default(),
                bindings![KeyCode::KeyZ],
            ),
        ]),
    ));
}

#[derive(Resource, Deref)]
pub struct CursorGrabOffset(pub Option<Vec2>);

#[derive(Resource, Deref, DerefMut)]
pub struct CursorGrabToggle(pub bool);

pub fn cursor_grab(
    mut offset: ResMut<CursorGrabOffset>,
    mut windows: Query<(&mut Window, &mut CursorOptions)>,
    free_cursor: Query<&Action<FreeCursor>>,
    toggle_cursor: Query<&ActionEvents, With<Action<ToggleCursor>>>,
    grabbers: Query<(), Or<(With<Actions<FlyingCamera>>, With<Actions<PlayerInput>>)>>,
    mut toggle: ResMut<CursorGrabToggle>,
) -> Result<()> {
    let free_cursor = free_cursor.single().unwrap();
    let toggle_cursor = toggle_cursor.single().unwrap();
    let grabbers = grabbers.iter().count();

    if toggle_cursor.contains(ActionEvents::FIRED) {
        toggle.0 = !toggle.0;
        info!("toggled cursor: {:?}", toggle.0);
    }

    for (mut window, mut cursor_options) in &mut windows {
        let mut grab = false;
        if window.focused {
            // Is there anything that wants control of the cursor?
            if grabbers > 0 {
                grab = true;
            }

            if **free_cursor {
                grab = false
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

        if grab && cursor_options.grab_mode == CursorGrabMode::None {
            cursor_options.grab_mode = CursorGrabMode::Locked;
            cursor_options.visible = false;

            let center = window.resolution.size() / 2.0;
            offset.0 = window.cursor_position().map(|current| current - center);
            window.set_cursor_position(Some(center)); // TODO: Figure out a way to ignore this for camera movement.
        } else if !grab && cursor_options.grab_mode == CursorGrabMode::Locked {
            cursor_options.grab_mode = CursorGrabMode::None;
            cursor_options.visible = true;
        }
    }

    Ok(())
}
