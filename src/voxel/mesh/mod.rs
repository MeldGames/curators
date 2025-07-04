use bevy::prelude::*;

pub mod binary_greedy;
pub mod surface_net;

#[derive(SystemSet, Copy, Clone, Debug, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct UpdateVoxelMeshSet;

pub fn plugin(app: &mut App) {
    app.add_plugins(surface_net::SurfaceNetPlugin);
    // app.add_plugins(ass_mesh::ASSMeshPlugin);
    // app.add_plugins(meshem::MeshemPlugin);
    // app.add_plugins(binary_greedy::plugin);
}
