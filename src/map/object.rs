use std::cmp::Ordering;

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::map::{Aabb, Digsite, VoxelAabb, WorldGenSet};

#[derive(Event)]
pub struct GenerateObjects {
    digsite: Entity,
    objects: Vec<Aabb>, // clone these entity prefabs
}

pub fn plugin(app: &mut App) {
    app.add_event::<GenerateObjects>();

    app.add_systems(PreUpdate, generate_objects.in_set(WorldGenSet::Objects));
    app.add_systems(Startup, create_digsite);
}

pub fn create_digsite(mut commands: Commands, mut writer: EventWriter<GenerateObjects>) {
    let digsite = commands
        .spawn(
            (Digsite {
                voxel_aabbs: vec![VoxelAabb::from_size(IVec3::ONE, IVec3::new(10, 10, 10))],
            }),
        )
        .id();

    writer.write(GenerateObjects {
        digsite,
        objects: vec![
            VoxelAabb::from_size(IVec3::ZERO, IVec3::ONE).as_vec3(),
            VoxelAabb::from_size(IVec3::ZERO, IVec3::new(1, 10, 1)).as_vec3(),
        ],
    });
}

pub fn generate_objects(
    mut generate_objects: EventReader<GenerateObjects>,
    digsites: Query<(&Digsite,)>,
    name: Query<NameOrEntity>,
) {
    for mut event in generate_objects.read() {
        let Ok(digsite) = digsites.get(event.digsite) else {
            warn!("digsite {:?}", name.get(event.digsite).unwrap());
            continue;
        };

        let mut object_list = event.objects.clone();
        // Sort by largest volume to smallest
        // This'll give us the greatest chance at finding positions for each
        // object. object_list.sort_by(by_volume);
    }
}

pub fn by_volume(a: &Aabb, b: &Aabb) -> Ordering {
    a.volume().partial_cmp(&b.volume()).unwrap_or(Ordering::Less)
}

pub struct VolumeWeights {
    weights: Vec<(i32, VoxelAabb)>,
}

impl Digsite {
    /// Randomly sample a position for an object's aabb.
    ///
    /// Make sure to rotate the object's aabb to what it will be before calling
    /// this.
    pub fn volume_weights(&self) -> VolumeWeights {
        let mut volume_weights = Vec::new();
        let mut volume_sum = 0;
        for aabb in &self.voxel_aabbs {
            volume_weights.push((volume_sum + aabb.volume(), *aabb));
            volume_sum += aabb.volume();
        }

        VolumeWeights { weights: volume_weights }
    }

    pub fn place_aabbs(&self, sizes: Vec<Vec3>) -> Vec<Vec3> {
        let mut sorted_sizes = sizes.iter().enumerate().collect::<Vec<_>>();
        sorted_sizes.sort_by(|(_, a), (_, b)| {
            a.length_squared().partial_cmp(&b.length_squared()).unwrap_or(Ordering::Less)
        });

        let weights = self.volume_weights();

        let mut placed = vec![Vec3::ZERO; sizes.len()];
        for (index, size) in sorted_sizes {
            // self.place_size(size);

            // placed[]
        }

        placed
    }
}
