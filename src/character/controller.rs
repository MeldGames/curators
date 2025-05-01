//! Use inputs to affect the kinematic controller's velocity/position

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use super::input::{Jump, Move, PlayerInput};
use super::kinematic::{KCCGravity, KinematicCharacterController};

// Marker component for whether or not we're currently grounded.
#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;

pub(super) fn plugin(app: &mut App) {
    app.add_observer(apply_movement).add_observer(apply_jump);

    app.add_systems(FixedUpdate, (velocity_dampening, update_grounded).chain());
}

pub fn apply_movement(
    trigger: Trigger<Fired<Move>>,
    mut players: Query<&mut KinematicCharacterController>,
) {
    let mut controller = players.get_mut(trigger.entity()).unwrap();
    let speed = 5.0;
    controller.velocity.x = trigger.value.x * speed;
    controller.velocity.z = -trigger.value.y * speed;

    if controller.velocity.y.is_nan() {
        controller.velocity.y = 0.0;
    }
}

pub fn apply_jump(
    trigger: Trigger<Fired<Jump>>,
    mut players: Query<(&mut KCCGravity, Has<Grounded>)>,
) {
    let (mut gravity, grounded) = players.get_mut(trigger.entity()).unwrap();
    if grounded {
        gravity.current_velocity = Vec3::Y * 100.0;
    }
}

pub fn velocity_dampening(mut query: Query<&mut KinematicCharacterController>, _time: Res<Time>) {
    for mut kcc in query.iter_mut() {
        kcc.velocity.x *= 0.9;
        kcc.velocity.z *= 0.9;
    }
}

pub fn update_grounded(
    mut commands: Commands,
    mut query: Query<(Entity, &ShapeHits, &Rotation), (With<KCCGravity>, With<RigidBody>)>,
) {
    // let _ = 45.0f32.to_radians();
    for (entity, hits, _) in &mut query {
        let is_grounded = hits.iter().any(|hit| true && hit.entity != entity);

        if is_grounded {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}
