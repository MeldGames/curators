use avian3d::prelude::*;
use bevy::prelude::*;

// pub mod avian;
pub mod boxes;
pub mod trimesh;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(PhysicsPlugins::default());
    app.add_plugins(PhysicsDebugPlugin::default());

    // app.add_plugins(boxes::VoxelBoxColliderPlugin);
    app.add_plugins(trimesh::VoxelTrimeshColliderPlugin);
}
