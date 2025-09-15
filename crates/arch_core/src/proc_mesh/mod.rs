//! Some procedural generation of meshing

use bevy::prelude::*;

pub mod fence;
pub mod character;

pub fn plugin(app: &mut App) {
    app.add_plugins(fence::plugin);
    app.add_plugins(character::plugin);
}
