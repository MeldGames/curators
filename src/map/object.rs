use avian3d::prelude::*;
use bevy::prelude::*;

use crate::map::Digsite;

#[derive(Event)]
pub struct GenerateObjects {
    digsite: Entity,
    object_probabilities: Vec<Entity>, // clone these entity prefabs
}

pub fn plugin(app: &mut App) {
    app.add_event::<GenerateObjects>();

    app.add_systems(PreUpdate, generate_objects);
}

pub fn generate_objects(
    mut generate_objects: EventReader<GenerateObjects>,
    digsites: Query<(&Digsite,)>,
    name: Query<NameOrEntity>,
) {
    for event in generate_objects.read() {
        let Ok(digsite) = digsites.get(event.digsite) else {
            warn!("digsite {:?}", name.get(event.digsite).unwrap());
            continue;
        };
    }
}
