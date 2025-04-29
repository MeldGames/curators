use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::{EnhancedInputPlugin, prelude::*};

#[derive(Component)]
pub struct Controlling;

#[derive(Component, InputContext)]
#[input_context(priority = 0)]
pub struct PlayerInput;

pub(super) fn plugin(app: &mut App) {
    app.add_input_context::<PlayerInput>();

    app.add_observer(player_binding);
}

pub fn player_binding(
    trigger: Trigger<Binding<PlayerInput>>,
    mut players: Query<&mut Actions<PlayerInput>>,
) {
    let Ok(mut actions) = players.get_mut(trigger.entity()) else {
        return;
    };

    info!("player binding");
    actions.bind::<Move>()
        .to(Cardinal::wasd_keys())
        .to(Cardinal::arrow_keys());
    actions.bind::<Jump>().to(KeyCode::Space);
}

#[derive(Debug, InputAction)]
#[input_action(output = Vec2)]
pub struct Move;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct Jump;
