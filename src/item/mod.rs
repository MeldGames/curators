use avian3d::prelude::*;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use bevy_mod_outline::{AsyncSceneInheritOutline, OutlineVolume};

#[derive(Component)]
#[require(SweptCcd, SleepingDisabled)]
pub struct Item;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Hold {
    /// Entity we are currently holding
    pub entity: Option<Entity>,

    // Hold entity,
    pub hold_entity: Entity,
    // TODO: Grab point?
    // pub local_grab_point: Vec3,
}

#[derive(Debug, InputContext)]
pub struct HandsFree;

#[derive(Debug, InputAction)]
#[input_action(output = bool, require_reset = true)]
pub struct Grab;

#[derive(Debug, InputContext)]
pub struct Holding;

#[derive(Debug, InputAction)]
#[input_action(output = bool, require_reset = true)]
pub struct Drop;

pub fn plugin(app: &mut App) {
    app.add_input_context::<HandsFree>().add_input_context::<Holding>();

    app.register_type::<Hold>();

    app.add_observer(add_hold)
        .add_observer(holding_binding)
        .add_observer(free_binding)
        .add_observer(drop_item);

    app.add_systems(Startup, spawn_test_items);
    app.add_systems(Update, grab_item).add_systems(Update, ItemOutline::lerp_color);
}

pub fn spawn_test_items(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Item,
        Name::new("Test item (sphere)"),
        Mesh3d(meshes.add(Sphere::new(0.3))),
        MeshMaterial3d(
            materials
                .add(StandardMaterial { base_color: Color::WHITE.into(), ..Default::default() }),
        ),
        RigidBody::Dynamic,
        Collider::sphere(0.3),
        Transform::from_xyz(2.0, 7.0, 2.0),
    ));

    commands.spawn((
        Item,
        Name::new("Test item (sphere)"),
        Mesh3d(meshes.add(Sphere::new(0.3))),
        MeshMaterial3d(
            materials
                .add(StandardMaterial { base_color: Color::WHITE.into(), ..Default::default() }),
        ),
        RigidBody::Dynamic,
        Collider::sphere(0.3),
        Transform::from_xyz(3.0, 7.0, 2.0),
    ));
}

pub fn add_hold(trigger: Trigger<OnInsert, Hold>, hold: Query<&Hold>, mut commands: Commands) {
    let Ok(hold) = hold.get(trigger.target()) else {
        return;
    };

    if hold.entity.is_some() {
        commands
            .entity(trigger.target())
            .remove::<Actions<HandsFree>>()
            .insert(Actions::<Holding>::default());
    } else {
        commands
            .entity(trigger.target())
            .remove::<Actions<Holding>>()
            .insert(Actions::<HandsFree>::default());
    }
}

pub fn holding_binding(
    trigger: Trigger<Bind<Holding>>,
    mut inputs: Query<(&mut Actions<Holding>, &mut Hold)>,
) {
    let Ok((mut actions, mut hold)) = inputs.get_mut(trigger.target()) else {
        return;
    };

    // info!("binding holding");
    actions.bind::<Drop>().to(KeyCode::KeyG).with_conditions(Press::default());
}

pub fn free_binding(
    trigger: Trigger<Bind<HandsFree>>,
    mut inputs: Query<(&mut Actions<HandsFree>, &mut Hold)>,
) {
    let Ok((mut actions, mut hold)) = inputs.get_mut(trigger.target()) else {
        return;
    };

    // info!("binding hands free");
    actions.bind::<Grab>().to(KeyCode::KeyG).with_conditions(Press::default());
}

#[derive(Component)]
pub struct ItemOutline {
    pub alpha_range: std::ops::RangeInclusive<f32>, // current alpha
    pub step: f32,                                  // step amount per second
    pub direction: bool,                            // true -> up, false -> down
}

impl Default for ItemOutline {
    fn default() -> Self {
        Self { alpha_range: 0.7..=1.0, step: 2.0, direction: true }
    }
}

impl ItemOutline {
    pub fn lerp_color(
        mut outlines: Query<(&mut OutlineVolume, &mut ItemOutline)>,
        time: Res<Time>,
    ) {
        for (mut volume, mut meta) in &mut outlines {
            let mut alpha = volume.colour.alpha();
            let step = meta.step * (meta.alpha_range.end() - meta.alpha_range.start());

            if meta.direction {
                alpha += step * time.delta_secs();
                if alpha > *meta.alpha_range.end() {
                    let over = alpha - *meta.alpha_range.end();
                    alpha = *meta.alpha_range.end() - over;
                    meta.direction = false;
                }
            } else {
                alpha -= step * time.delta_secs();
                if alpha < *meta.alpha_range.start() {
                    let over = alpha - *meta.alpha_range.start();
                    alpha = *meta.alpha_range.start() - over;
                    meta.direction = true;
                }
            }

            volume.colour.set_alpha(alpha);
        }
    }
}

pub fn grab_item(
    mut holding: Query<(Entity, NameOrEntity, &mut Hold, &Actions<HandsFree>)>,
    globals: Query<&GlobalTransform>,
    mut items: Query<(Entity, NameOrEntity, &mut Transform, &Item)>,
    outlined_items: Query<Entity, (With<Item>, With<OutlineVolume>)>,
    child_of: Query<(), With<ChildOf>>,
    mut commands: Commands,
) -> Result<()> {
    let mut remove_outlines = outlined_items.iter().collect::<HashSet<_>>();
    let mut add_outlines = HashSet::new();

    for (holder_entity, holder_name, mut hold, actions) in holding {
        let Ok(hold_position) = globals.get(hold.hold_entity) else {
            continue;
        };

        // Find item within range
        //
        // TODO:
        // - Take into account direction of holder
        // - Take into account closeness of items
        let mut closest_item = None;
        const PICKUP_RADIUS: f32 = 2.0;
        for (item_entity, _, item_transform, _) in &items {
            let distance = item_transform.translation.distance(hold_position.translation());
            if distance <= PICKUP_RADIUS {
                if child_of.contains(item_entity) {
                    warn!("item entity already has a parent");
                    continue;
                }

                let mut set = if let Some((_, closest_distance)) = closest_item {
                    distance < closest_distance
                } else {
                    true
                };

                if set {
                    closest_item = Some((item_entity, distance));
                }
            }
        }

        if let Some((item_entity, _)) = closest_item {
            let Ok((_, item_name, mut item_transform, item)) = items.get_mut(item_entity) else {
                continue;
            };

            if actions.value::<Grab>()? {
                info!("{} picked up {}", holder_name, item_name);
                hold.entity = Some(item_entity);
                item_transform.translation = Vec3::ZERO;
                commands.entity(item_entity).insert((
                    ColliderDisabled,
                    RigidBodyDisabled,
                    ChildOf(hold.hold_entity),
                ));

                commands
                    .entity(holder_entity)
                    .remove::<Actions<HandsFree>>()
                    .insert(Actions::<Holding>::default());
            } else {
                if remove_outlines.contains(&item_entity) {
                    remove_outlines.remove(&item_entity);
                } else {
                    add_outlines.insert(item_entity);
                }
            }
        }
    }

    for entity in remove_outlines {
        commands.entity(entity).remove::<OutlineVolume>();
    }

    for entity in add_outlines {
        commands.entity(entity).insert((
            ItemOutline { alpha_range: 0.4..=0.7, step: 2.0, ..default() },
            OutlineVolume { visible: true, width: 4.0, colour: Color::srgba(0.0, 0.0, 0.0, 0.4) },
        ));
    }

    Ok(())
}

pub fn drop_item(
    trigger: Trigger<Fired<Drop>>,
    mut holding: Query<(Entity, &mut Hold)>,
    mut transforms: Query<(&mut Transform, &GlobalTransform)>,
    name: Query<NameOrEntity>,
    mut commands: Commands,
) {
    let Ok((holder_entity, mut hold)) = holding.get_mut(trigger.target()) else {
        return;
    };

    let holder_name = name.get(holder_entity);
    let item_name = hold.entity.map(|item_entity| name.get(item_entity).unwrap());
    info!("{} dropped {}", holder_name.unwrap(), item_name.unwrap());

    // TODO: Align dropped item to the grid if targetting something.
    if let Some(item_entity) = hold.entity.take() {
        if let Ok((mut transform, global)) = transforms.get_mut(item_entity) {
            transform.translation = global.translation();
        }

        commands
            .entity(item_entity)
            .remove::<ColliderDisabled>()
            .remove::<RigidBodyDisabled>()
            .remove::<ChildOf>();
    }

    commands
        .entity(holder_entity)
        .remove::<Actions<Holding>>()
        .insert(Actions::<HandsFree>::default());
}
