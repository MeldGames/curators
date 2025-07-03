use avian3d::prelude::*;
use bevy::color::palettes::css::GRAY;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::experimental::taa::TemporalAntiAliasing;
use bevy::core_pipeline::fxaa::Fxaa;
use bevy::core_pipeline::smaa::{Smaa, SmaaPreset};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::{
    Atmosphere, ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel,
    ShadowFilteringMethod,
};
use bevy::prelude::*;
use bevy_edge_detection::EdgeDetection;
use bevy_enhanced_input::prelude::*;

use super::input::DigState;
use crate::camera::*;
use crate::item::Hold;

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
    asset_server: Res<AssetServer>,
) {
    let collider = Collider::capsule(0.4, 0.8);
    let mesh = meshes.add(Mesh::from(Capsule3d::new(0.4, 0.8)));

    let hold_entity = commands
        .spawn((
            Name::new("Hold position"),
            Transform { translation: Vec3::NEG_Z, scale: Vec3::splat(0.5), ..default() },
        ))
        .id();

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
            DigState::default(),
            Hold { entity: None, hold_entity },
        ))
        .with_child((
            Name::new("Player Spotlight"),
            Transform::from_xyz(0.0, 3.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
            SpotLight {
                color: Color::srgb(1.0, 1.0, 1.0),
                intensity: 7_500.0,
                shadows_enabled: false,
                ..default()
            },
        ))
        .add_child(hold_entity)
        .id();

    let metering_mask: Handle<Image> = asset_server.load("basic_metering_mask.png");

    let camera_components = (
        Camera { hdr: true, ..default() },
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection::default()),
        Tonemapping::default(),
        Atmosphere::EARTH,
        // Exposure::SUNLIGHT,
        // Bloom::NATURAL,
        /*bevy::core_pipeline::auto_exposure::AutoExposure {
            range: -3.0..=3.0,
            // range: -9.0..=1.0,
            filter: 0.10..=0.90,
            speed_brighten: 3.0, // 3.0 default
            speed_darken: 1.0,   // 1.0 default
            // metering_mask: metering_mask.clone(),
            ..default()
        },*/
        // ShadowFilteringMethod::Temporal,
        Msaa::Off,
        // TemporalAntiAliasing::default(),
        ScreenSpaceAmbientOcclusion {
            quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
            constant_object_thickness: 4.0,
        },
        EdgeDetection {
            depth_threshold: 0.3,
            normal_threshold: 1.0,
            depth_thickness: 1.0,
            edge_color: Color::srgba(0.0, 0.0, 0.0, 0.5),
            enable_depth: true,
            enable_normal: true,
            enable_color: false,

            uv_distortion_frequency: Vec2::new(1.0, 1.0),
            uv_distortion_strength: Vec2::new(0.0, 0.0),
            ..default()
        },
        Smaa { preset: SmaaPreset::Ultra },
        // Fxaa::default(),
    );

    let flying = commands
        .spawn((
            Name::new("Flying camera"),
            FlyingSettings::default(),
            FlyingState::default(),
            camera_components.clone(),
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
            camera_components.clone(),
            Transform::from_translation(Vec3::new(8.0, 10.0, 8.0))
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ))
        .id();

    // let digsite = commands
    //     .spawn((
    //         Name::new("Digsite camera"),
    //         DigsiteSettings::default(),
    //         DigsiteState::default(),
    //         camera_components.clone(),
    //         Transform::from_translation(Vec3::new(8.0, 10.0, 8.0))
    //             .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    //     ))
    //     .id();

    commands.spawn((
        Name::new("Camera toggle"),
        Actions::<CameraToggle>::default(),
        CameraEntities { flying, follow, /*digsite,*/ active: ActiveCamera::Flying },
    ));
}
