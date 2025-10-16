#![allow(unused)]
#![allow(unused_imports)]

use std::cmp::Ordering;

use avian3d::prelude::*;
use bevy::ecs::entity_disabling::Disabled;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_prng::WyRand;
use bevy_rand::prelude::*;
use rand::Rng;
use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;
use rand::seq::WeightError;

use crate::map::{Aabb, Digsite, DigsiteObject, VoxelAabb, WorldGenSet};
use crate::voxel::GRID_SCALE;

#[derive(Message)]
pub struct GenerateObjects {
    digsite: Entity,
    objects: Vec<Vec3>, // clone these entity prefabs
}

pub fn plugin(app: &mut App) {
    app.add_message::<GenerateObjects>();

    app.add_systems(PreUpdate, generate_objects.in_set(WorldGenSet::Objects));
    app.add_systems(Startup, create_test_digsite);
}

pub fn create_test_digsite(mut commands: Commands, mut writer: MessageWriter<GenerateObjects>) {
    let digsite = commands
        .spawn(
            (Digsite {
                voxel_aabbs: vec![VoxelAabb {
                    min: IVec3::new(0, 30, 0),
                    max: IVec3::new(10, 48, 10),
                }],
                ..default()
            }),
        )
        .id();

    writer.write(GenerateObjects { digsite, objects: vec![Vec3::ONE, Vec3::new(1.0, 10.0, 1.0)] });
}

pub fn generate_objects(
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    mut generate_objects: MessageReader<GenerateObjects>,
    digsites: Query<(&Digsite,)>,
    name: Query<NameOrEntity>,
) {
    for mut event in generate_objects.read() {
        let Ok((digsite,)) = digsites.get(event.digsite) else {
            warn!("digsite {:?}", name.get(event.digsite).unwrap());
            continue;
        };

        // TODO: Read https://docs.rs/bevy_rand/latest/bevy_rand/
        let mut object_list = event.objects.clone();
        // let positions = digsite.place_aabbs(object_list, &mut rng);
        // Sort by largest volume to smallest
        // This'll give us the greatest chance at finding positions for each
        // object. object_list.sort_by(by_volume);
    }
}

pub fn by_volume(a: &Aabb, b: &Aabb) -> Ordering {
    a.volume().partial_cmp(&b.volume()).unwrap_or(Ordering::Less)
}

impl Digsite {
    /// Randomly sample a position for an object's aabb.
    ///
    /// Make sure to rotate the object's aabb to what it will be before calling
    /// this.
    pub fn volume_weighted_index(&self) -> Result<WeightedIndex<i32>, WeightError> {
        let mut weights = Vec::new();
        for aabb in &self.voxel_aabbs {
            weights.push(aabb.volume());
        }

        WeightedIndex::new(weights)
    }

    pub fn random_volume(&self, rng: &mut impl Rng) -> VoxelAabb {
        let index = self.volume_weighted_index().unwrap();
        self.voxel_aabbs[index.sample(rng)]
    }

    pub fn place_objects(&self, objects: Vec<DigsiteObject>, rng: &mut impl Rng) -> Vec<Vec3> {
        let mut sorted_sizes = objects.iter().enumerate().collect::<Vec<_>>();
        sorted_sizes.sort_by(|(_, a), (_, b)| {
            a.volume().partial_cmp(&b.volume()).unwrap_or(Ordering::Less)
        });

        let mut placed = vec![Vec3::ZERO; objects.len()];
        for (index, object) in sorted_sizes {
            for _ in 0..3 {
                let volume = self.random_volume(&mut *rng).as_vec3();
                let object_aabb = object.local_aabb();
                let Some(fitting_zone) = object_aabb.fitting_zone(&volume) else {
                    continue;
                };

                let point = fitting_zone.random_point(&mut *rng);
                // TODO: check for collision?
                placed[index] = point;
                break;
            }
        }

        placed
    }
}
