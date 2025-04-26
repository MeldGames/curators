
pub mod grid;
pub mod border;

pub use grid::*;

use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(grid::plugin)
        .add_plugins(border::plugin);
}