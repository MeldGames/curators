//! Create borders/ground around the voxel grid.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::voxel::GRID_SCALE;
use crate::voxel::voxel_grid::VoxelGrid;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, rebuild_borders);
}

#[derive(Component)]
pub struct Border;

pub fn rebuild_borders(
    mut commands: Commands,
    digsite: Query<(&GlobalTransform, &VoxelGrid), Changed<VoxelGrid>>,
    borders: Query<Entity, With<Border>>,
    mut last_size: Local<[i32; 3]>,
) {
    let Ok((digsite_transform, digsite)) = digsite.get_single() else {
        return;
    };

    // Voxel grid has changed, check if the size has.
    let new_size = digsite.array();
    if new_size == *last_size {
        return;
    }
    *last_size = new_size;

    // Clear old borders
    for entity in &borders {
        commands.entity(entity).despawn();
    }

    // Create new borders around digsite
    const PADDING: f32 = 20.0;
    let digsite_bounds = digsite.scaled_bounds();
    let ground_level = digsite.ground_level() as f32 * GRID_SCALE.y;
    let y_pos = ground_level / 2.0;
    let y_height = ground_level;

    // left ground
    commands.spawn((
        Transform::from_xyz(-PADDING / 2.0, y_pos, digsite_bounds.z / 2.0),
        Collider::cuboid(PADDING, y_height, digsite_bounds.z + PADDING * 2.0),
    ));
    // right ground
    commands.spawn((
        Transform::from_xyz(digsite_bounds.x + PADDING / 2.0, y_pos, digsite_bounds.z / 2.0),
        Collider::cuboid(PADDING, y_height, digsite_bounds.z + PADDING * 2.0),
    ));

    // backward ground
    commands.spawn((
        Transform::from_xyz(digsite_bounds.x / 2.0, y_pos, digsite_bounds.z + PADDING / 2.0),
        Collider::cuboid(digsite_bounds.x, y_height, PADDING),
    ));

    // forward ground
    commands.spawn((
        Transform::from_xyz(digsite_bounds.x / 2.0, y_pos, -PADDING / 2.0),
        Collider::cuboid(digsite_bounds.x, y_height, PADDING),
    ));
}
