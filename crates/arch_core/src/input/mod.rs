//! Configure BEI and such.

use bevy::prelude::*;
use bevy_egui::input::EguiWantsInput;
use bevy_egui::{EguiContexts, egui};
use bevy_enhanced_input::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_systems(PreUpdate, (disable_mouse, disable_keyboard).before(EnhancedInputSet::Update));
}

pub fn disable_mouse(
    mut action_sources: ResMut<ActionSources>,
    interactions: Query<&Interaction>,
    egui_wants: Option<Res<EguiWantsInput>>,
) {
    let Some(egui_wants) = egui_wants else { return; };
    // bevy ui
    let bevy_ui_using = interactions.iter().any(|&interaction| interaction != Interaction::None);
    // egui
    let egui_using = egui_wants.wants_any_pointer_input();

    let using = bevy_ui_using || egui_using;

    action_sources.mouse_buttons = !using;
    action_sources.mouse_wheel = !using;
}

pub fn disable_keyboard(
    mut action_sources: ResMut<ActionSources>,
    // interactions: Query<&Interaction>,
    egui_wants: Option<Res<EguiWantsInput>>,
) {
    let Some(egui_wants) = egui_wants else { return; };
    // bevy ui
    // let bevy_ui_using = interactions.iter().all(|&interaction| interaction ==
    // Interaction::None); egui
    let egui_using = egui_wants.wants_any_keyboard_input();

    let using = egui_using;

    action_sources.keyboard = !using;
}
