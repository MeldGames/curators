use avian3d::prelude::*;
use bevy::color::palettes::css::GRAY;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::camera::*;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_player);
}

#[derive(Component)]
#[require(Name::new("Player"))]
pub struct Player;

pub fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let collider = Collider::capsule(0.4, 0.8);
    let mesh = meshes.add(Mesh::from(Capsule3d::new(0.4, 0.8)));

    let player = commands
        .spawn((
            Player,
            Transform::from_xyz(0.0, 10.0, 0.0),
            collider.clone(),
            super::kinematic::KinematicCharacterController {
                up: Vec3::Y,
                collider: collider.clone(),
                ..default()
            },
            Mesh3d(mesh),
            MeshMaterial3d(
                materials.add(StandardMaterial { base_color: GRAY.into(), ..Default::default() }),
            ),
            Actions::<super::input::PlayerInput>::default(),
        ))
        .id();

    let flying = commands
        .spawn((
            Name::new("Flying camera"),
            FlyingSettings::default(),
            FlyingState::default(),
            Camera { hdr: true, ..default() },
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection::default()),
            Tonemapping::default(),
            Transform::from_translation(Vec3::new(8.0, 10.0, 8.0))
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ))
        .id();

    let follow = commands
        .spawn((
            Name::new("Follow camera"),
            FollowSettings::default(),
            FollowState::default(),
            FollowPlayer(player),
            Camera { hdr: true, ..default() },
            Camera3d::default(),
            Tonemapping::default(),
            Projection::Perspective(PerspectiveProjection::default()),
            Transform::from_translation(Vec3::new(8.0, 10.0, 8.0))
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ))
        .id();

    let digsite = commands
        .spawn((
            Name::new("Digsite camera"),
            DigsiteSettings::default(),
            DigsiteState::default(),
            Camera { hdr: true, ..default() },
            Camera3d::default(),
            Tonemapping::default(),
            Projection::Perspective(PerspectiveProjection::default()),
            Transform::from_translation(Vec3::new(8.0, 10.0, 8.0))
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ))
        .id();

    commands.spawn((
        Name::new("Camera toggle"),
        Actions::<CameraToggle>::default(),
        CameraEntities { flying, follow, digsite, active: ActiveCamera::Digsite },
    ));
}
