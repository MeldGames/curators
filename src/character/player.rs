use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::camera::*;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_player);
}

#[derive(Component)]
#[require(Name(|| Name::new("Player")))]
pub struct Player;

pub fn spawn_player(mut commands: Commands) {
    let collider = Collider::capsule(0.4, 0.8);
    let player = commands.spawn((
        Player,
        Transform::from_xyz(0.0, 10.0, 0.0),
        collider.clone(),
        super::kinematic::KinematicCharacterController {
            up: Vec3::Y,
            collider: collider.clone(),
            ..default()
        },
        //Actions::<super::input::PlayerInput>::default(),
    )).id();

        let flying = commands.spawn((
            Name::new("Flying camera"),
            FlyingSettings::default(),
            FlyingState::default(),
            Camera { ..default() },
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection::default()),
            Transform::from_translation(Vec3::new(8.0, 10.0, 8.0))
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        )).id();

        let follow = commands.spawn((
            Name::new("Follow camera"),
            FollowSettings::default(),
            FollowState::default(),
            FollowPlayer(player),
            Camera { ..default() },
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection::default()),
            Transform::from_translation(Vec3::new(8.0, 10.0, 8.0))
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        )).id();

        let digsite = commands.spawn((
            Name::new("Digsite camera"),
            DigsiteSettings::default(),
            DigsiteState::default(),
            Camera { ..default() },
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection::default()),
            Transform::from_translation(Vec3::new(8.0, 10.0, 8.0))
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        )).id();


        commands.spawn((
            Name::new("Camera toggle"),
            Actions::<CameraToggle>::default(),
            CameraEntities {
                flying: flying,
                follow: follow,
                digsite: digsite,
                active: ActiveCamera::Digsite,
            }
        ));
}
