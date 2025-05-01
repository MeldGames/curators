pub mod border;
pub mod grid;

use bevy::prelude::*;
pub use grid::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(grid::plugin).add_plugins(border::plugin);
}
