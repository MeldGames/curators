use std::f32::consts::PI;

use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.insert_resource(AmbientLight { brightness: 50.0, ..default() });
    // app.add_systems(Startup, setup);
    // app.add_systems(Update, update);
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        DirectionalLight { shadows_enabled: true, ..default() },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, PI * -0.15, PI * -0.15)),
    ));

    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.5),
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(0.0, 10.0, 1.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(0.0, 9.0, 0.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(material),
        Transform::from_xyz(1.0, 10.0, 0.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.4).mesh().uv(72, 36))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.4, 0.4),
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            ..default()
        })),
        SphereMarker,
        Transform::from_xyz(0.0, 10.0, 0.0),
    ));
}

#[derive(Component)]
pub struct SphereMarker;

fn update(mut sphere: Single<&mut Transform, With<SphereMarker>>, time: Res<Time>) {
    sphere.translation.y = ops::sin(time.elapsed_secs() / 1.7) * 0.7 + 10.0;
}
