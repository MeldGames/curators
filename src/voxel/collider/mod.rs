use avian3d::prelude::*;
use bevy::prelude::*;

pub mod avian;
pub mod boxes;

pub struct VoxelColliderPlugin;
impl Plugin for VoxelColliderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default());
        app.add_plugins(PhysicsDebugPlugin::default());

        //app.add_plugins(boxes::VoxelBoxColliderPlugin);
    }
}
