use avian3d::prelude::*;
use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_player);
}

#[derive(Component)]
#[require(Name(|| Name::new("Player")))]
pub struct Player;

pub fn spawn_player(mut commands: Commands) {
    let collider = Collider::capsule(0.5, 1.0);
    commands.spawn((
        Player,
        Transform::from_xyz(0.0, 10.0, 0.0),
        collider.clone(),
        RigidBody::Kinematic,
        super::kinematic::KinematicCharacterController {
            up: Vec3::Y,
            collider: collider.clone(),
            ..default()
        },
    ));
}
