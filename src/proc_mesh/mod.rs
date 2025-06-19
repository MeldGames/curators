//! Some procedural generation of meshing

use bevy::prelude::*;

pub mod fence;

pub fn plugin(mut app: &mut App) {
    app.add_plugins(fence::plugin);
}
