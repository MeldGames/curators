use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::voxel::{GRID_SCALE, Voxel, VoxelChunk, Voxels};

#[derive(Component)]
pub struct Controlling;

#[derive(Component, InputContext)]
#[input_context(priority = 0)]
pub struct PlayerInput;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<DigState>();

    app.add_input_context::<PlayerInput>();

    app.add_observer(player_binding);

    app.add_systems(Update, (dig_target, dig_block).chain());
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
    actions.bind::<Dig>().to(KeyCode::KeyE).to(MouseButton::Left);
}

#[derive(Debug, InputAction)]
#[input_action(output = Vec2)]
pub struct Move;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct Jump;

/// Dig depends on the movement of the character.
/// Moving north will have you mine the blocks north of you, moving east mines
/// the blocks east of you. Standing still is how you can dig straight down.
///
/// We should probably have it "stick" so if you start mining while moving
/// north, you stay digging north, rather than switch to digging down.
#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct Dig;

#[derive(Component, Debug, Reflect)]
pub struct DigState {
    // (Digsite entity, voxel position)
    pub target_block: Option<(Entity, [i32; 3])>,
    /// How long it takes to trigger one dig.
    pub dig_time: f32,
    /// How long we've held down dig.
    pub time_since_dig: f32,
}

impl Default for DigState {
    fn default() -> Self {
        Self { target_block: None, dig_time: 0.1, time_since_dig: 0.0 }
    }
}

pub fn dig_target(
    mut players: Query<(&GlobalTransform, &Actions<PlayerInput>, &mut DigState, &Collider)>,
    mut voxels: Query<(Entity, &GlobalTransform, &mut Voxels)>,
    time: Res<Time>,
    mut gizmos: Gizmos,
) {
    for (global_transform, actions, mut state, collider) in &mut players {
        let interact = actions.get::<Dig>().unwrap();
        match interact.state() {
            ActionState::Fired => {
                state.time_since_dig += time.delta_secs();
            },
            ActionState::None => {
                state.time_since_dig = 0.0;
            },
            _ => {},
        }

        // Find a target digsite and block position
        for (digsite_entity, digsite_transform, grid) in &voxels {
            if let Some(hit) = grid.cast_ray(
                digsite_transform,
                Ray3d { origin: global_transform.translation(), direction: Dir3::NEG_Y },
                f32::INFINITY,
            ) {
                // TODO: Character height + X blocks
                let collider_aabb = collider.aabb(Vec3::ZERO, Quat::IDENTITY);
                let character_ground = collider_aabb.size().y / 2.0;
                const BLOCKS_DOWN: f32 = 5.0;
                let max_down_distance = character_ground + BLOCKS_DOWN * GRID_SCALE.y;
                // info!("down_distance: {:?}", max_down_distance);
                // info!("hit.distance: {:?}", hit.distance);

                // TODO: Fix raycast hit.distance for scaling
                if hit.distance < max_down_distance {
                    state.target_block = Some((digsite_entity, Into::<[i32; 3]>::into(hit.voxel)));
                    break;
                }
            }
        }

        if let Some((digsite_entity, voxel)) = state.target_block {
            if let Ok((_, digsite_transform, mut grid)) = voxels.get_mut(digsite_entity) {
                let voxel: Vec3 = IVec3::from(voxel).as_vec3();
                let voxel_point = digsite_transform.transform_point(voxel);
                gizmos.cuboid(
                    Transform {
                        translation: voxel_point + (Vec3::ONE * GRID_SCALE) / 2.0,
                        scale: GRID_SCALE * 1.01,
                        rotation: Quat::IDENTITY,
                    },
                    Color::srgb(1.0, 1.0, 1.0),
                );
            }
        }
    }
}

pub fn dig_block(
    mut players: Query<(&Actions<PlayerInput>, &mut DigState)>,
    mut voxels: Query<&mut Voxels>,
) {
    for (actions, mut dig_state) in &mut players {
        if let ActionState::Fired = actions.get::<Dig>().unwrap().state() {
            if let Some((digsite_entity, voxel_pos)) = dig_state.target_block {
                if let Ok(mut voxels) = voxels.get_mut(digsite_entity) {
                    if let Some(voxel_state) = voxels.get_voxel(voxel_pos.into()) {
                        if dig_state.time_since_dig >= dig_state.dig_time {
                            let dig_power = 10;

                            if let Some(health) = voxels.health(voxel_pos.into()) {
                                let new_health = health.saturating_sub(dig_power);
                                if new_health == 0 {
                                    voxels.set_voxel(voxel_pos.into(), Voxel::Air);
                                } else {
                                    voxels.set_health(voxel_pos.into(), new_health);
                                }

                                dig_state.time_since_dig -= dig_state.dig_time;
                            }
                        }
                    }
                }
            }
        }
    }
}
