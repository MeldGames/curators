use std::cmp::Ordering;

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::map::{Aabb, VoxelAabb, Digsite, WorldGenSet};

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

pub fn create_digsite(
    mut commands: Commands,
    mut writer: EventWriter<GenerateObjects>,
) {
    let digsite = commands.spawn(
        (
            Digsite {
                voxel_aabbs: vec![VoxelAabb::from_size(IVec3::ONE, IVec3::new(10, 10, 10))]
            }
        )
    ).id();

    writer.write(GenerateObjects {
        digsite: digsite,
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
        // This'll give us the greatest chance at finding positions for each object.
        //object_list.sort_by(by_volume); 

    }
}

pub fn by_volume(a: &VoxelAabb, b: &VoxelAabb) -> Ordering {
    a.volume().cmp(&b.volume())
}

impl Digsite {

    /// Randomly sample a position for an object's aabb.
    /// 
    /// Make sure to rotate the object's aabb to what it will be before calling this.
    pub fn sample_object_position(&self, object: &Aabb) -> Vec3 {

        for aabb in &self.voxel_aabbs {
            let aabb = aabb.as_vec3();
        }

        Vec3::ZERO
    }
}
