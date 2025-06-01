use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

#[derive(Component)]
pub struct Item;

#[derive(Component)]
pub struct Hold(Option<Entity>);

#[derive(Debug, InputContext)]
pub struct HandsFree;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct Grab;

#[derive(Debug, InputContext)]
pub struct Holding;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct Drop;

pub fn plugin(app: &mut App) {
    app.add_observer(holding_binding)
        .add_observer(free_binding)
        .add_observer(grab_item)
        .add_observer(drop_item);
}

pub fn holding_binding(
    trigger: Trigger<Binding<Holding>>,
    mut inputs: Query<&mut Actions<Holding>>,
) {
    let Ok(mut actions) = inputs.get_mut(trigger.target()) else {
        return;
    };

    actions.bind::<Drop>().to(KeyCode::KeyG);
}

pub fn free_binding(
    trigger: Trigger<Binding<HandsFree>>,
    mut inputs: Query<&mut Actions<HandsFree>>,
) {
    let Ok(mut actions) = inputs.get_mut(trigger.target()) else {
        return;
    };

    actions.bind::<Grab>().to(KeyCode::KeyG);
}

pub fn grab_item(
    trigger: Trigger<Fired<Grab>>,
    mut holding: Query<(&GlobalTransform, &mut Hold)>,
    items: Query<(Entity, &GlobalTransform, &Item)>,
) {
    let Ok((holder_transform, mut hold)) = holding.get_mut(trigger.target()) else {
        return;
    };

    // Find item within range
    //
    // TODO:
    // - Take into account direction of holder
    // - Take into account closeness of items
    for (item_entity, item_transform, item) in items {
        const PICKUP_RADIUS: f32 = 3.0;
        if item_transform.translation().distance(holder_transform.translation()) < PICKUP_RADIUS {
            hold.0 = Some(item_entity);
            return;
        }
    }
}

pub fn drop_item(trigger: Trigger<Fired<Drop>>, mut holding: Query<&mut Hold>) {
    let Ok(mut hold) = holding.get_mut(trigger.target()) else {
        return;
    };

    hold.0 = None;
}
