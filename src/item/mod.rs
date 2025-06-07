use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

#[derive(Component)]
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
    app
        .add_input_context::<HandsFree>()
        .add_input_context::<Holding>();

    app.register_type::<Hold>();

    app
        .add_observer(add_hold)
        .add_observer(holding_binding)
        .add_observer(free_binding)
        .add_observer(grab_item)
        .add_observer(drop_item);

    app.add_systems(Startup, spawn_test_items);
}

pub fn spawn_test_items(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    meshes.add(Sphere::new(0.5));
    commands.spawn((
        Item,
        Name::new("Test item (sphere)"),
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE.into(),
            ..Default::default()
        })),

        Transform::from_xyz(2.0, 7.0, 2.0),
    ));
}

pub fn add_hold(
    trigger: Trigger<OnInsert, Hold>,
    hold: Query<&Hold>,
    mut commands: Commands,
) {
    let Ok(hold) = hold.get(trigger.target()) else {
        return;
    };

    if hold.entity.is_some() {
        commands.entity(trigger.target())
            .remove::<Actions<HandsFree>>()
            .insert(Actions::<Holding>::default());
    } else {
        commands.entity(trigger.target())
            .remove::<Actions<Holding>>()
            .insert(Actions::<HandsFree>::default());
    }
}


pub fn holding_binding(
    trigger: Trigger<Binding<Holding>>,
    mut inputs: Query<(&mut Actions<Holding>, &mut Hold)>,
) {
    let Ok((mut actions, mut hold)) = inputs.get_mut(trigger.target()) else {
        return;
    };

    info!("binding holding");
    actions.bind::<Drop>().to(KeyCode::KeyG).with_conditions(Press::default());
}

pub fn free_binding(
    trigger: Trigger<Binding<HandsFree>>,
    mut inputs: Query<(&mut Actions<HandsFree>, &mut Hold)>,
) {
    let Ok((mut actions, mut hold)) = inputs.get_mut(trigger.target()) else {
        return;
    };

    info!("binding hands free");
    actions.bind::<Grab>().to(KeyCode::KeyG).with_conditions(Press::default());
}

pub fn grab_item(
    trigger: Trigger<Fired<Grab>>,
    mut holding: Query<(Entity, NameOrEntity, &GlobalTransform, &mut Hold)>,
    mut items: Query<(Entity, NameOrEntity, &mut Transform, &Item)>,
    child_of: Query<(), With<ChildOf>>,
    mut commands: Commands,
) {
    let Ok((holder_entity, holder_name, holder_transform, mut hold)) = holding.get_mut(trigger.target()) else {
        return;
    };

    // Find item within range
    //
    // TODO:
    // - Take into account direction of holder
    // - Take into account closeness of items
    for (item_entity, item_name, mut item_transform, item) in &mut items {
        const PICKUP_RADIUS: f32 = 3.0;
        if item_transform.translation.distance(holder_transform.translation()) < PICKUP_RADIUS {
            if child_of.contains(item_entity) {
                warn!("item entity already has a parent");
                continue;
            }

            info!("{} picked up {}", holder_name, item_name);
            hold.entity = Some(item_entity);
            item_transform.translation = Vec3::ZERO;
            commands.entity(item_entity)
                .insert(ChildOf(hold.hold_entity));

            commands.entity(holder_entity)
                .remove::<Actions<HandsFree>>()
                .insert(Actions::<Holding>::default());

            return;
        }
    }
}

pub fn drop_item(trigger: Trigger<Fired<Drop>>, mut holding: Query<(Entity, &mut Hold)>, mut transforms: Query<(&mut Transform, &GlobalTransform)>, name: Query<NameOrEntity>, mut commands: Commands) {
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

        commands.entity(item_entity).remove::<ChildOf>();
    }

    commands.entity(holder_entity)
        .remove::<Actions<Holding>>()
        .insert(Actions::<HandsFree>::default());
}
