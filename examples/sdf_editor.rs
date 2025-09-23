//! SDF editor

use arch_core::sdf::{self, Sdf, SdfNode};
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

#[derive(Component, Reflect)]
#[require(Transform::default(), GlobalTransform::default(), Visibility::Inherited)]
pub struct SdfMesh(pub sdf::SdfNode);

pub fn main() {
    let mut app = App::new();
    app.register_type::<SdfMesh>();
    app.add_plugins(arch::core::sdf::register_sdf_reflect_types);
    arch::core::viewer(&mut app);
    app.insert_resource(AmbientLight { brightness: 2500.0, ..default() });
    app.add_systems(Startup, create_sdf);
    app.add_systems(PreUpdate, remesh_sdf);
    app.run();
}

pub fn create_sdf(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.spawn((
        Name::new("SDF"),
        MeshMaterial3d(materials.add(StandardMaterial {
            perceptual_roughness: 0.9,
            base_color: Color::srgb(1.0, 0.0, 0.0),
            ..default()
        })),
        SdfMesh(SdfNode::default()),
        Transform::default(),
        GlobalTransform::default(),
        Visibility::Inherited,
    ));
}

pub fn remesh_sdf(
    mut commands: Commands,
    mut sdfs: Query<(Entity, &mut Transform, &SdfMesh), Changed<SdfMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (sdf_entity, mut transform, sdf) in &mut sdfs {
        let aabb =
            sdf.0.aabb().unwrap_or(Aabb3d { min: Vec3A::splat(-10.0), max: Vec3A::splat(10.0) });
        let step_amount = 0.01;
        let sample_epsilon = Vec3::splat(step_amount * 4.0);
        let min = Vec3::from(aabb.min) - sample_epsilon;
        let max = Vec3::from(aabb.max) + sample_epsilon;
        if let Some((mesh, buffer)) =
            arch_core::proc_mesh::character::sdf_to_mesh(&sdf.0, min, max, step_amount)
        {
            commands.entity(sdf_entity).insert(Mesh3d(meshes.add(mesh)));
        } else {
            commands.entity(sdf_entity).remove::<Mesh3d>();
        }
    }
}


// Snarl node editor
