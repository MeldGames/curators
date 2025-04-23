use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_player);
}

#[derive(Component)]
#[require(Name(|| Name::new("Player")))]
pub struct Player;

pub fn spawn_player(mut commands: Commands) {
    let collider = Collider::capsule(0.4, 0.8);
    commands.spawn((
        Player,
        Transform::from_xyz(0.0, 10.0, 0.0),
        collider.clone(),
        super::kinematic::KinematicCharacterController {
            up: Vec3::Y,
            collider: collider.clone(),
            ..default()
        },
        Actions::<super::input::PlayerInput>::default(),
    ));
}
