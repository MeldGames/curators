//! Basic following camera for a specific Entity.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

pub fn plugin(app: &mut App) {
    app.register_type::<FollowPlayer>();

    app.add_input_context::<FollowCamera>();

    app.add_systems(PostUpdate, follow_player.after(PhysicsSet::Sync));
}

#[derive(Component)]
pub struct FollowCamera;

// No actions for follow camera right now.

#[derive(Component, Debug, Reflect)]
pub struct FollowPlayer(pub Entity);

#[derive(Component, Debug, Reflect)]
pub struct FollowSettings {
    pub offset: Vec3,
}

impl Default for FollowSettings {
    fn default() -> Self {
        Self { offset: Vec3::new(0.0, 2.0, 0.4) * 7.0 }
    }
}

#[derive(Component, Debug, Reflect, Default)]
pub struct FollowState;

pub fn follow_player(
    cameras: Query<(Entity, &FollowPlayer, &FollowSettings), With<Actions<FollowCamera>>>,
    mut transforms: Query<&mut Transform>,
) {
    for (entity, player, settings) in &cameras {
        let player_transform = transforms.get(player.0).unwrap().clone();
        let mut camera_transform = transforms.get_mut(entity).unwrap();
        camera_transform.translation = player_transform.translation + settings.offset;
        camera_transform.look_at(player_transform.translation, Vec3::Y);
    }
}
