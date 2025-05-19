//! Game camera, floats far enough away from the digsite to show a view of the
//! whole digsite.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::voxel::GRID_SCALE;
use crate::voxel::voxel_grid::VoxelGrid;

pub fn plugin(app: &mut App) {
    app.register_type::<DigsiteEntity>();

    app.add_input_context::<DigsiteCamera>();

    app.add_systems(Update, (attach_digsite, follow_digsite).chain());
}

#[derive(InputContext)]
#[input_context(priority = 10)]
pub struct DigsiteCamera;

// No actions for digsite camera right now.
// TODO:
// - Mouse wheel zoom in out?
// - Change viewing angle?

/// Digsite (voxel grid) entity we want to center on.
#[derive(Component, Debug, Reflect)]
pub struct DigsiteEntity(pub Entity);

#[derive(Component, Debug, Reflect)]
pub struct DigsiteSettings {
    pub offset: Vec3,
}

impl Default for DigsiteSettings {
    fn default() -> Self {
        Self { offset: Vec3::new(0.0, 2.0, 1.0) * 8.0 }
    }
}

#[derive(Component, Debug, Reflect, Default)]
pub struct DigsiteState;

pub fn attach_digsite(
    mut commands: Commands,
    cameras: Query<(Entity, &DigsiteSettings), Without<DigsiteEntity>>,
    grid: Query<Entity, With<VoxelGrid>>,
) {
    let Some(grid) = grid.iter().next() else {
        return;
    };

    for (entity, _) in &cameras {
        commands.entity(entity).insert(DigsiteEntity(grid));
    }
}

pub fn follow_digsite(
    cameras: Query<
        (Entity, &DigsiteEntity, &DigsiteSettings),
        Or<(Changed<DigsiteSettings>, Changed<DigsiteEntity>)>,
    >,
    mut transforms: Query<&mut Transform>,
    grid: Query<&VoxelGrid>,
) {
    for (entity, digsite, settings) in &cameras {
        let Ok(digsite_transform) = transforms.get(digsite.0).cloned() else {
            continue;
        };

        let grid = grid.get(digsite.0).unwrap();
        let grid_bounds = Into::<IVec3>::into(grid.array()).as_vec3() * GRID_SCALE;
        let shift_up = Vec3::Z * 3.0;
        let center = digsite_transform.translation + grid_bounds / 2.0;
        let target_point = center + shift_up;

        let mut camera_transform = transforms.get_mut(entity).unwrap();
        camera_transform.translation = target_point + settings.offset;
        camera_transform.look_at(target_point, Vec3::Y);
    }
}
