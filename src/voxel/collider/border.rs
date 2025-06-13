//! Create borders/ground around the voxel grid.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::voxel::{GRID_SCALE, Voxels};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, rebuild_borders);
}

#[derive(Component)]
#[require(Name::new("Border"))]
pub struct Border;

pub fn rebuild_borders(
    mut commands: Commands,
    voxels: Query<(&GlobalTransform, &Voxels), Changed<Voxels>>,
    borders: Query<Entity, With<Border>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,

    mut last_bounds: Local<(IVec3, IVec3)>,
) {
    let Ok((_voxels_transform, voxels)) = voxels.single() else {
        return;
    };

    // Voxel grid has changed, check if the size has.
    let new_bounds = voxels.voxel_bounds();
    if new_bounds == *last_bounds {
        return;
    }
    *last_bounds = new_bounds;

    // Clear old borders
    for entity in &borders {
        commands.entity(entity).despawn();
    }

    // Create new borders around voxels
    const PADDING: f32 = 20.0;
    let (min, max) = new_bounds;
    let voxels_extents = (max.as_vec3() - min.as_vec3());
    info!("min: {:?}, max: {:?}", min, max);

    let voxels_bounds = voxels_extents * GRID_SCALE;
    let ground_level = 16.0 * GRID_SCALE.y;
    let y_pos = ground_level / 2.0;
    let y_height = ground_level;

    let mut from_lengths = |x, y, z| {
        (
            RigidBody::Static,
            Collider::cuboid(x, y, z),
            Mesh3d(meshes.add(Mesh::from(Cuboid::new(x, y, z)))),
        )
    };

    let ground_material = MeshMaterial3d(materials.add(StandardMaterial {
        base_color: Srgba::new(0.0, 82.0 / 255.0, 0.0, 1.0).into(),
        perceptual_roughness: 1.0,
        // reflectance: 0.0,
        ..Default::default()
    }));

    const GROUND_CATCH_HEIGHT: f32 = PADDING;
    // ground catch
    commands.spawn((
        Border,
        Name::new("Ground catch"),
        Transform::from_xyz(voxels_bounds.x / 2.0, -GROUND_CATCH_HEIGHT / 2.0, voxels_bounds.z / 2.0),
        from_lengths(voxels_bounds.x, GROUND_CATCH_HEIGHT, voxels_bounds.z),
        ground_material.clone(),
    ));

    // left ground
    commands.spawn((
        Border,
        Transform::from_xyz(-PADDING / 2.0, y_pos, voxels_bounds.z / 2.0),
        from_lengths(PADDING, y_height, voxels_bounds.z + PADDING * 2.0),
        ground_material.clone(),
    ));
    // right ground
    commands.spawn((
        Border,
        Transform::from_xyz(voxels_bounds.x + PADDING / 2.0, y_pos, voxels_bounds.z / 2.0),
        from_lengths(PADDING, y_height, voxels_bounds.z + PADDING * 2.0),
        ground_material.clone(),
    ));

    // backward ground
    commands.spawn((
        Border,
        Transform::from_xyz(voxels_bounds.x / 2.0, y_pos, voxels_bounds.z + PADDING / 2.0),
        from_lengths(voxels_bounds.x, y_height, PADDING),
        ground_material.clone(),
    ));

    // forward ground
    commands.spawn((
        Border,
        Transform::from_xyz(voxels_bounds.x / 2.0, y_pos, -PADDING / 2.0),
        from_lengths(voxels_bounds.x, y_height, PADDING),
        ground_material.clone(),
    ));
}
