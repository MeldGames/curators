use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::EnhancedInputPlugin;
use bevy_enhanced_input::prelude::*;

use crate::voxel::voxel_grid::VoxelGrid;
use crate::voxel::GRID_SCALE;

#[derive(Component)]
pub struct Controlling;

#[derive(Component, InputContext)]
#[input_context(priority = 0)]
pub struct PlayerInput;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<DigState>();

    app.add_input_context::<PlayerInput>();

    app.add_observer(player_binding);

    app.add_systems(Update, dig);
}

pub fn player_binding(
    trigger: Trigger<Binding<PlayerInput>>,
    mut players: Query<&mut Actions<PlayerInput>>,
) {
    let Ok(mut actions) = players.get_mut(trigger.target()) else {
        return;
    };

    actions.bind::<Move>().to(Cardinal::wasd_keys()).to(Cardinal::arrow_keys());
    actions.bind::<Jump>().to(KeyCode::Space);
    actions.bind::<Dig>()
        .to(KeyCode::KeyE)
        .to(MouseButton::Left);
}

#[derive(Debug, InputAction)]
#[input_action(output = Vec2)]
pub struct Move;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct Jump;

/// Dig depends on the movement of the character.
/// Moving north will have you mine the blocks north of you, moving east mines the blocks east of you.
/// Standing still is how you can dig straight down.
/// 
/// We should probably have it "stick" so if you start mining while moving north, you stay digging north,
/// rather than switch to digging down.
#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct Dig;

#[derive(Component, Debug, Reflect)]
pub struct DigState {
    // (Digsite entity, voxel position)
    pub target_block: Option<(Entity, [i32; 3])>,
}

impl Default for DigState {
    fn default() -> Self {
        Self {
            target_block: None,
        }
    }
}

pub fn dig(mut players: Query<(&GlobalTransform, &Actions<PlayerInput>, &mut DigState)>, mut digsites: Query<(Entity, &GlobalTransform, &mut VoxelGrid)>, mut gizmos: Gizmos) {
    for (global_transform, actions, mut state) in &mut players {
        let interact = actions.action::<Dig>();
        match interact.state() {
            ActionState::Fired => {
                if state.target_block.is_none() {
                }
            }
            ActionState::None => {
                state.target_block = None;
            }
            _ => {},
        }

        // Find a target digsite and block position
        for (digsite_entity, digsite_transform, grid)  in &digsites {
            if let Some(hit) = grid.cast_ray(digsite_transform, Ray3d {
                origin: global_transform.translation(),
                direction: Dir3::NEG_Y,
            }) {
                if hit.distance < 5.0 {
                    state.target_block = Some((digsite_entity, Into::<[i32; 3]>::into(hit.voxel)));
                    break;
                }
            }
        }

        if let Some((digsite_entity, voxel)) = state.target_block {
            if let Ok((_, digsite_transform, mut grid)) = digsites.get_mut(digsite_entity) {
                let voxel: Vec3 = IVec3::from(voxel).as_vec3();
                let voxel_point = digsite_transform.transform_point(voxel);
                gizmos.cuboid(Transform {
                    translation: voxel_point + (Vec3::ONE * GRID_SCALE) / 2.0,
                    scale: GRID_SCALE * 1.01,
                    rotation: Quat::IDENTITY,
                }, Color::srgb(1.0, 1.0, 1.0));
            }
        }
    }
}