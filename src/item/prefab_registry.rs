use avian3d::prelude::*;
use bevy::ecs::entity::EntityClonerBuilder;
use bevy::ecs::entity_disabling::Disabled;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::map::DigsiteObject;

pub fn plugin(app: &mut App) {
    app.register_type::<Prefabs>().register_type::<Prefab>();

    app.insert_resource(Prefabs::new());

    app.add_observer(add_prefabs);

    app.add_systems(Startup, spawn_object_prefabs);
}

#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct Prefabs {
    pub prefabs: HashMap<String, Entity>,
}

impl Prefabs {
    pub fn new() -> Self {
        Self { prefabs: HashMap::new() }
    }

    pub fn add_prefab(&mut self, tag: String, entity: Entity) {
        self.prefabs.insert(tag, entity);
    }

    pub fn spawn<'a, 'b: 'a>(
        &self,
        commands: &'b mut Commands,
        tag: impl AsRef<String>,
    ) -> Option<EntityCommands<'a>> {
        let Some(entity) = self.prefabs.get(tag.as_ref()).copied() else {
            return None;
        };

        let mut e = commands.entity(entity);
        e.clone_and_spawn_with(|builder| {
            builder.deny::<Disabled>();
            builder.deny::<Prefab>();
        });
        Some(e)
    }

    pub fn spawn_with<'a, 'b: 'a>(
        &self,
        commands: &'b mut Commands,
        tag: impl AsRef<String>,
        config: impl FnOnce(&mut EntityClonerBuilder) + Send + Sync + 'static,
    ) -> Option<EntityCommands<'a>> {
        let Some(entity) = self.prefabs.get(tag.as_ref()).copied() else {
            return None;
        };

        let mut e = commands.entity(entity);
        e.clone_and_spawn_with(|builder| {
            builder.deny::<Disabled>();
            builder.deny::<Prefab>();
            config(builder);
        });
        Some(e)
    }
}

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Clone)]
#[require(Disabled)]
pub struct Prefab {
    pub tag: String,
}

pub fn spawn_object_prefabs(mut commands: Commands) {
    // Orb of Pondering.
    commands.spawn((
        Name::new("THE GREAT ORB OF PONDERING"),
        DigsiteObject { size: Vec3::new(1.0, 1.0, 1.0) },
        Collider::sphere(0.5),
        Transform::default(),
        RigidBody::Dynamic,
        Prefab { tag: "item/orb_of_pondering".to_owned() },
    ));
}

pub fn add_prefabs(
    added: Trigger<OnAdd, Prefab>,
    mut registry: ResMut<Prefabs>,
    prefabs: Query<&Prefab, With<Disabled>>,
) {
    let prefab_entity = added.target();
    let Ok(prefab) = prefabs.get(prefab_entity) else {
        return;
    };

    registry.add_prefab(prefab.tag.clone(), prefab_entity);
}
