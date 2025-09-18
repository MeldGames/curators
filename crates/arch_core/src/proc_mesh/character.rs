//! Create our character's mesh procedurally out of a couple of primitives as
//! SDFs.

use bevy::asset::RenderAssetUsages;
use bevy::ecs::schedule::Stepping;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy_math::bounding::Aabb3d;
use fast_surface_nets::ndshape::{RuntimeShape, Shape};
use fast_surface_nets::{SurfaceNetsBuffer, surface_nets};

use crate::sdf::{self, *};

pub fn plugin(app: &mut App) {
    app.register_type::<CharacterMeshSettings>();
    app.add_systems(Startup, spawn_character_mesh);
    app.add_systems(PreUpdate, update_character_mesh);
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct CharacterMeshSettings {
    pub body_fatness: Vec3,

    pub head_fatness: Vec3,
    pub head_height: f32,

    pub neck_z_connection: f32,
    pub neck_fatness: f32,

    pub eye_radius: f32,
    // eye_radius + eye_padding = subtraction from head
    pub eye_padding: f32,
    pub eye_offset: Vec3,
}

impl Default for CharacterMeshSettings {
    fn default() -> Self {
        let body_fatness = Vec3::new(1.0, 0.8, 1.0);
        let head_fatness = Vec3::new(0.2, 0.2, 0.15);
        Self { body_fatness, head_fatness, head_height: 2.0, neck_z_connection: body_fatness.z * -0.75, neck_fatness: 0.25, eye_radius: 0.15, eye_padding: 0.05, eye_offset: Vec3::new(0.0, 0.1, -0.1) }
    }
}

pub fn update_character_mesh(
    mut commands: Commands,
    characters: Query<(Entity, &CharacterMeshSettings), Changed<CharacterMeshSettings>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, settings) in &characters {
        let body = sdf::Ellipsoid { radii: settings.body_fatness };

        let neck = sdf::Capsule {
            start: Vec3::new(0.0, 0.0, settings.neck_z_connection),
            end: Vec3::new(0.0, settings.head_height, settings.neck_z_connection),
            radius: 0.25,
        };

        let head =
            sdf::Ellipsoid { radii: settings.head_fatness }
                .translate(Vec3::new(
                    0.0,
                    settings.head_height,
                    settings.neck_z_connection - settings.head_fatness.z,
                ));

        // eyes
        let socket = sdf::Sphere { radius: settings.eye_radius + settings.eye_padding }.translate(settings.eye_offset);

        let head = head.smooth_subtraction(socket, 0.1);

        let body_neck_join = body.smooth_union(neck, 0.3);
        let head_neck_join = body_neck_join.smooth_union(head, 0.3);

        let sdf = head_neck_join;

        let aabb =
            sdf.aabb().unwrap_or(Aabb3d { min: Vec3A::splat(-10.0), max: Vec3A::splat(10.0) });
        let step_amount = 0.1;
        let sample_epsilon = Vec3::splat(step_amount * 4.0);
        let min = Vec3::from(aabb.min) - sample_epsilon;
        let max = Vec3::from(aabb.max) + sample_epsilon;
        if let Some(mut mesh) = sdf_to_mesh(sdf, min, max, step_amount) {
            mesh.duplicate_vertices();
            mesh.compute_flat_normals();
            commands.entity(entity).insert(Mesh3d(meshes.add(mesh)));
        }
    }
}

pub fn spawn_character_mesh(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Character mesh"),
        CharacterMeshSettings { ..default() },
        Transform { translation: Vec3::new(-3.0, 5.0, -3.0), ..default() },
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 0.0),
            perceptual_roughness: 0.9,
            ..default()
        })),
    ));
}

pub fn samples(
    sdf: impl Sdf,
    sample_min: Vec3,
    sample_max: Vec3,
    step_amount: f32,
) -> (RuntimeShape<u32, 3>, Vec<f32>) {
    info!("sample_min: {sample_min}, sample_max: {sample_max}, step_amount: {step_amount}");
    let sample_width = sample_max - sample_min;
    let steps_f32 = sample_width / Vec3::splat(step_amount as f32);
    info!("steps_f32: {steps_f32}");

    let steps = steps_f32.round().as_ivec3();
    info!("steps: {steps}");
    let shape = RuntimeShape::<u32, 3>::new([steps.x as u32, steps.y as u32, steps.z as u32]);
    let shape_size = shape.size() as usize;
    let steps_size = steps.x as usize * steps.y as usize * steps.z as usize;
    assert_eq!(shape_size, steps_size);

    let mut samples = vec![1.0f32; steps_size];
    for x in 0..steps.x {
        for y in 0..steps.y {
            for z in 0..steps.z {
                let offset = Vec3::new(x as f32, y as f32, z as f32) * step_amount;
                let sample_point = sample_min + offset;
                let distance = sdf.sdf(sample_point);
                let index = shape.linearize([x as u32, y as u32, z as u32]);

                samples[index as usize] = distance;
            }
        }
    }

    (shape, samples)
}

pub fn sdf_to_mesh(
    sdf: impl Sdf,
    sample_min: Vec3,
    sample_max: Vec3,
    step_amount: f32,
) -> Option<Mesh> {
    let size = sample_max - sample_min;
    if size.x <= step_amount || size.y <= step_amount || size.z <= step_amount {
        return None;
    }

    // Sample the SDF into a grid
    let (shape, samples) = samples(sdf, sample_min, sample_max, step_amount);

    info!("shape array: {:?}", shape.as_array());
    info!("shape size: {:?}", shape.size());
    let mut buffer = SurfaceNetsBuffer::default();
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    surface_nets(&samples.as_slice(), &shape, [0; 3], shape.as_array().map(|n| n - 2), &mut buffer);

    for position in &mut buffer.positions {
        // This is not quite correct. To adjust the vertex positions to match the AABB
        // defined by sample_min and sample_max, you should first scale the
        // grid-space position by step_amount, then add sample_min to shift into world
        // space.
        *position = position.map(|n| n * step_amount);
        *position = (Vec3::from(*position) + sample_min).into();
    }

    for normal in &mut buffer.normals {
        *normal = Vec3::from(*normal).normalize_or_zero().into();
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, buffer.positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, buffer.normals);

    mesh.insert_indices(Indices::U32(buffer.indices));

    Some(mesh)
}
