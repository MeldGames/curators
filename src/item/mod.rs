use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

#[derive(Component)]
pub struct Item;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct Hold {
    /// Entity we are currently holding
    pub entity: Option<Entity>,

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

    app
        .add_systems(FixedPostUpdate, attach_item)
        .add_systems(PostUpdate, attach_item);
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
    items: Query<(Entity, NameOrEntity, &GlobalTransform, &Item)>,
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
    for (item_entity, item_name, item_transform, item) in items {
        const PICKUP_RADIUS: f32 = 3.0;
        if item_transform.translation().distance(holder_transform.translation()) < PICKUP_RADIUS {
            info!("{} picked up {}", holder_name, item_name);
            hold.entity = Some(item_entity);
            commands.entity(holder_entity)
                .remove::<Actions<HandsFree>>()
                .insert(Actions::<Holding>::default());

            return;
        }
    }
}

pub fn drop_item(trigger: Trigger<Fired<Drop>>, mut holding: Query<(Entity, &mut Hold)>, name: Query<NameOrEntity>, mut commands: Commands) {
    let Ok((holder_entity, mut hold)) = holding.get_mut(trigger.target()) else {
        return;
    };

    let holder_name = name.get(holder_entity);
    let item_name = hold.entity.map(|item_entity| name.get(item_entity).unwrap());
    info!("{} dropped {}", holder_name.unwrap(), item_name.unwrap());

    // TODO: Align dropped item to the grid if targetting something.
    hold.entity = None;
    commands.entity(holder_entity)
        .remove::<Actions<Holding>>()
        .insert(Actions::<HandsFree>::default());
}

/// Attach item to the player when held
pub fn attach_item(holders: Query<(Entity, &Hold)>, mut transforms: Query<&mut Transform>, globals: Query<&GlobalTransform>) {
    for (holder_entity, hold) in &holders {
        if let Some(holding) = hold.entity {
            let Ok(holder_global) = globals.get(holder_entity)  else { continue; };
            let Ok(mut item_transform) = transforms.get_mut(holding) else { continue; };
            item_transform.translation = holder_global.translation();
        }
    }

}