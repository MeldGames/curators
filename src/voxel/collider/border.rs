//! Create borders/ground around the voxel grid.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::map::Aabb;
use crate::voxel::{VoxelAabb, Voxels};

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

    mut last_bounds: Local<VoxelAabb>,
) {
    let Ok((_voxels_transform, voxels)) = voxels.single() else {
        return;
    };

    // Voxel grid has changed, check if the size has.
    let aabb = voxels.voxel_aabb();
    if aabb == *last_bounds {
        return;
    }
    *last_bounds = aabb;

    // Clear old borders
    for entity in &borders {
        commands.entity(entity).despawn();
    }

    // Create new borders around voxels
    const PADDING: f32 = 20.0;
    let aabb = aabb.as_vec3().correct();
    info!("aabb: {:?}", aabb);
    info!("aabb.center: {:?}", aabb.center());
    info!("aabb.size: {:?}", aabb.size());

    // let ground_level = 16.0 * GRID_SCALE.y;
    // let y_pos = ground_level / 2.0;
    // let y_height = ground_level;

    let ground_material = MeshMaterial3d(materials.add(StandardMaterial {
        base_color: Srgba::new(0.0, 82.0 / 255.0, 0.0, 1.0).into(),
        perceptual_roughness: 1.0,
        // reflectance: 0.0,
        ..Default::default()
    }));

    let mut from_aabb = |aabb: Aabb| {
        let size = aabb.size() / 2.0;
        (
            RigidBody::Static,
            Collider::cuboid(size.x, size.y, size.z),
            Mesh3d(meshes.add(Mesh::from(Cuboid::new(size.x, size.y, size.z)))),
            Transform::from_translation(aabb.center()),
            ground_material.clone(),
        )
    };

    let ground_catch_aabb = Aabb {
        min: aabb.min - Vec3::Y * PADDING,
        max: Vec3::new(aabb.max.x, aabb.min.y, aabb.max.z),
    }
    .correct();

    // const GROUND_CATCH_HEIGHT: f32 = PADDING;
    // ground catch
    commands.spawn((Border, Name::new("Ground catch"), from_aabb(ground_catch_aabb)));
    // commands.spawn((Border, Name::new("Ground catch"), from_aabb(aabb)));

    // left ground
    // commands.spawn((
    //     Border,
    //     Transform::from_xyz(-PADDING / 2.0, y_pos, voxels_bounds.z / 2.0),
    //     from_lengths(PADDING, y_height, voxels_bounds.z + PADDING * 2.0),
    //     ground_material.clone(),
    // ));
    // // right ground
    // commands.spawn((
    //     Border,
    //     Transform::from_xyz(voxels_bounds.x + PADDING / 2.0, y_pos,
    // voxels_bounds.z / 2.0),     from_lengths(PADDING, y_height,
    // voxels_bounds.z + PADDING * 2.0),     ground_material.clone(),
    // ));

    // // backward ground
    // commands.spawn((
    //     Border,
    //     Transform::from_xyz(voxels_bounds.x / 2.0, y_pos, voxels_bounds.z +
    // PADDING / 2.0),     from_lengths(voxels_bounds.x, y_height, PADDING),
    //     ground_material.clone(),
    // ));

    // // forward ground
    // commands.spawn((
    //     Border,
    //     Transform::from_xyz(voxels_bounds.x / 2.0, y_pos, -PADDING / 2.0),
    //     from_lengths(voxels_bounds.x, y_height, PADDING),
    //     ground_material.clone(),
    // ));
}
