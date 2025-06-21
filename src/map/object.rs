use std::cmp::Ordering;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_rand::RngComponent;
use rand::Rng;

use crate::map::{Aabb, Digsite, VoxelAabb, WorldGenSet};
use crate::voxel::GRID_SCALE;

#[derive(Event)]
pub struct GenerateObjects {
    digsite: Entity,
    objects: Vec<Vec3>, // clone these entity prefabs
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

    writer.write(GenerateObjects { digsite, objects: vec![Vec3::ONE, Vec3::new(1, 10, 1)] });
}

pub fn generate_objects(
    mut generate_objects: EventReader<GenerateObjects>,
    digsites: Query<(&Digsite, &RngComponent)>,
    name: Query<NameOrEntity>,
) {
    for mut event in generate_objects.read() {
        let Ok((digsite, rng)) = digsites.get(event.digsite) else {
            warn!("digsite {:?}", name.get(event.digsite).unwrap());
            continue;
        };

        // TODO: Read https://docs.rs/bevy_rand/latest/bevy_rand/
        let mut object_list = event.objects.clone();
        digsite.place_aabbs(object_list, rng)
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
    max_volume: i32,
}

impl VolumeWeights {
    pub fn pick_volume(&self, size: Vec3, mut rng: impl Rng + Copy) -> VoxelAabb {
        // TODO: Filter volumes that are too small for the object.
        let volume_pick = rng.random_range(0..self.max_volume);

        for (starting_volume, aabb) in &self.weights {
            if volume_pick >= *starting_volume {
                return *aabb;
            }
        }

        unreachable!("volume was outside range: {volume_pick:?}");
    }

    pub fn pick_point(&self, size: Vec3, mut rng: impl Rng + Copy) -> Vec3 {
        let volume = self.pick_volume(size, rng).as_vec3();

        // Calculate the inner area that keeps the object inside the volume.
        let volume_size = volume.size();
        let variance = (volume_size - size).max(Vec3::ZERO);
        let min = volume.center() - variance / 2.0;
        let max = volume.center() + variance / 2.0;

        // Pick a random point in that variance.
        Vec3::new(
            rng.random_range(min.x..max.x),
            rng.random_range(min.y..max.y),
            rng.random_range(min.z..max.z),
        )
    }
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

        VolumeWeights { weights: volume_weights, max_volume: volume_sum }
    }

    pub fn place_aabbs(&self, sizes: Vec<Vec3>, rng: impl Rng + Copy) -> Vec<Vec3> {
        let mut sorted_sizes = sizes.iter().enumerate().collect::<Vec<_>>();
        sorted_sizes.sort_by(|(_, a), (_, b)| {
            a.length_squared().partial_cmp(&b.length_squared()).unwrap_or(Ordering::Less)
        });

        let weights = self.volume_weights();

        let mut placed = vec![Vec3::ZERO; sizes.len()];
        for (index, size) in sorted_sizes {
            let point = weights.pick_point(*size, rng);
            placed[index] = point;
            // TODO: check for collision
        }

        placed
    }
}
