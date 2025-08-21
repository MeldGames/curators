use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
pub use chunk::{Scalar, VoxelChunk, padded, unpadded};

use crate::voxel::{Voxel, VoxelAabb, Voxels};

// Data
pub mod chunk;

// Meshing
pub mod binary_greedy;
pub mod surface_net;

// Perf control
pub mod frustum_chunks;
pub mod lod;
pub mod remesh;

// Visual
pub mod camera_inside;

pub use binary_greedy::BinaryGreedy;
pub use remesh::Remesh;
pub use surface_net::SurfaceNet;

#[derive(SystemSet, Copy, Clone, Debug, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub enum UpdateVoxelMeshSet {
    Init,
    Spawn,
    Mesh,
    Finish,
}

pub fn plugin(app: &mut App) {
    app.add_event::<ChangedChunk>();

    // app.add_plugins(ass_mesh::ASSMeshPlugin);
    // app.add_plugins(meshem::MeshemPlugin);
    app.add_plugins(binary_greedy::plugin);
    app.add_plugins(surface_net::SurfaceNetPlugin);

    app.add_plugins(remesh::plugin);
    app.add_plugins(frustum_chunks::plugin);
    app.add_plugins(lod::plugin);
    // app.add_plugins(camera_inside::plugin);

    app.configure_sets(
        PostUpdate,
        (
            UpdateVoxelMeshSet::Init,
            UpdateVoxelMeshSet::Spawn,
            UpdateVoxelMeshSet::Mesh,
            UpdateVoxelMeshSet::Finish,
        )
            .chain(),
    );

    app.add_systems(PostUpdate, clear_changed_chunks.before(UpdateVoxelMeshSet::Finish));
}

#[derive(Event, Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct ChangedChunk {
    pub grid_entity: Entity,
    pub chunk_point: IVec3,
}

pub fn clear_changed_chunks(
    mut voxels: Query<(Entity, &mut Voxels)>,
    mut writer: EventWriter<ChangedChunk>,
) {
    for (voxel_entity, mut voxels) in &mut voxels {
        // writer.write(ChangedChunks {
        //     voxel_entity,
        //     changed_chunks:
        // voxels.render_chunks.changed_chunk_pos_iter().collect::<Vec<_>>(),
        // });
        // voxels.render_chunks.clear_changed_chunks();
    }
}
