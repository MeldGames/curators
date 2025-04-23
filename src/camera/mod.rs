use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

pub mod flying;
pub mod follow;

pub use flying::Flying;
pub use follow::Follow;

pub fn plugin(app: &mut App) {
    app.add_plugins(follow::plugin).add_plugins(flying::plugin);
}
