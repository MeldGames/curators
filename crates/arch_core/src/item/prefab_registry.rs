use avian3d::prelude::*;
use bevy::ecs::entity::{EntityClonerBuilder, OptOut};
use bevy::ecs::entity_disabling::Disabled;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::map::DigsiteObject;

pub fn plugin(app: &mut App) {
    app.register_type::<Prefabs>().register_type::<Prefab>();

    app.insert_resource(Prefabs::new());

    app.add_observer(add_prefabs);

    app.add_systems(Startup, spawn_object_prefabs);
    // app.add_systems(PreUpdate, spawn_at_cursor);
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
        tag: impl AsRef<str>,
    ) -> Option<EntityCommands<'a>> {
        let Some(entity) = self.prefabs.get(tag.as_ref()).copied() else {
            return None;
        };

        let mut e = commands.entity(entity);
        e.clone_and_spawn_with_opt_out(|builder| {
            builder.deny::<Disabled>();
            builder.deny::<Prefab>();
        });
        Some(e)
    }

    pub fn spawn_with<'a, 'b: 'a>(
        &self,
        commands: &'b mut Commands,
        tag: impl AsRef<String>,
        config: impl FnOnce(&mut EntityClonerBuilder<OptOut>) + Send + Sync + 'static,
    ) -> Option<EntityCommands<'a>> {
        let Some(entity) = self.prefabs.get(tag.as_ref()).copied() else {
            return None;
        };

        let mut e = commands.entity(entity);
        e.clone_and_spawn_with_opt_out(|builder| {
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

pub fn spawn_object_prefabs(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Orb of Pondering.
    commands.spawn((
        Prefab { tag: "item/orb_of_pondering".to_owned() },
        Name::new("THE GREAT ORB OF PONDERING"),
        crate::item::Item,
        DigsiteObject { size: Vec3::new(1.0, 1.0, 1.0) },
        Collider::sphere(0.5),
        Transform::default(),
        RigidBody::Dynamic,
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(
            materials
                .add(StandardMaterial { base_color: Color::WHITE.into(), ..Default::default() }),
        ),
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

pub fn spawn_at_cursor(
    mut commands: Commands,
    cursor: Res<crate::voxel::CursorVoxel>,
    input: Res<ButtonInput<MouseButton>>,
    prefabs: Res<Prefabs>,
) {
    let Some(hit) = cursor.hit() else {
        return;
    };

    if input.just_pressed(MouseButton::Left) {
        info!("spawning orb of pondering");
        prefabs
            .spawn(&mut commands, "item/orb_of_pondering")
            .unwrap()
            .insert(Transform { translation: hit.world_space + Vec3::Y * 3.0, ..default() });
    }
}
