//! A kinematic character controller framework inspired by the [bevy-tnua](https://github.com/idanarye/bevy-tnua/tree/main) project
//! While also taking inspiration and ideas from the [Avian Physics](https://discord.com/channels/691052431525675048/1124043933886976171) channel in the official Bevy Discord server.\
//!
//! Please note that all components within this module are prefixed with `KCC`
//! to make it clear that they are part of the Kinematic Character Controller
//! framework.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::character::input::{Jump, PlayerInput};

mod movement;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<KinematicCharacterController>()
        .register_type::<KCCFloorDetection>()
        .register_type::<KCCFloorSnap>()
        .register_type::<KCCGravity>()
        .register_type::<KCCGrounded>()
        .register_type::<KCCJump>()
        .register_type::<KCCSlope>();

    app.add_systems(
        FixedUpdate,
        (
            velocity_dampening,
            gravity_system,
            handle_jump,
            movement::collide_and_slide_system,
            update_kinematic_character_controller,
            update_kinematic_floor,
            floor_snap,
        )
            .chain(),
    );
}

/// A component that represents the core logic of a kinematic character
/// controller. This component has a dedicated system that updates its internal
/// state and calls the movement basis.
#[derive(Component, Reflect, Debug)]
#[require(
    RigidBody::Kinematic,
    KCCFloorDetection,
    KCCFloorSnap,
    KCCGravity,
    KCCGrounded,
    KCCSlope,
    KCCJump
)]
#[reflect(Component)]
pub struct KinematicCharacterController {
    /// The velocity we had last tick.
    pub prev_velocity: Vec3,
    /// The velocity we have this tick.
    pub velocity: Vec3,
    /// The up vector of the character.
    pub up: Vec3,
    /// How many times the collider will "bounce" off of surfaces.
    pub bounces: u32,
    /// The collider that represents the shape of this character.
    #[reflect(ignore)]
    pub collider: Collider,
}

impl Default for KinematicCharacterController {
    fn default() -> Self {
        Self {
            prev_velocity: Vec3::ZERO,
            velocity: Vec3::ZERO,
            up: Vec3::Y,
            bounces: 4,
            collider: Collider::capsule(0.4, 0.8),
        }
    }
}

#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component)]
/// A component that when added to the controller enables grounding management.
/// This component requires the [`KCCFloorDetection`] component to be present on
/// the same entity.
pub struct KCCGrounded {
    /// Is this character currently grounded?
    pub grounded: bool,
    /// Was this character grounded last tick?
    pub prev_grounded: bool,
}

/// Component that represents the floor detection of a kinematic character
/// controller. This component has a dedicated system that runs a shapecast to
/// detect the floor.
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct KCCFloorDetection {
    /// [`Vec3`] representing the normal of the floor we were on last tick.
    /// [`Vec3::ZERO`] if we are not grounded.
    pub prev_floor_normal: Vec3,
    /// [`Vec3`] representing the normal of the floor we are currently standing
    /// on. [`Vec3::ZERO`] if we are not grounded.
    pub floor_normal: Vec3,
    /// Direction that gravity is pulling this character in
    pub ground_direction: Vec3,
    #[reflect(ignore)]
    pub floor_collider: Collider,
    /// The distance from the floor that this character is currently at.
    pub floor_distance: f32,
    /// How far from the floor this character can be before it is considered not
    /// grounded.
    pub max_floor_distance: f32,
}

impl Default for KCCFloorDetection {
    fn default() -> Self {
        Self {
            prev_floor_normal: Vec3::ZERO,
            floor_normal: Vec3::ZERO,
            ground_direction: Vec3::NEG_Y,
            floor_collider: Collider::capsule(0.4, 0.8),
            floor_distance: 0.0,
            max_floor_distance: 0.05,
        }
    }
}

/// A component that when added to the controller enables snapping to the floor.
/// This component requires the [`KCCFloorDetection`] and the [`KCCGrounded`]
/// components to be present on the same entity.
#[derive(Component, Reflect, Debug, Default)]
#[reflect(Component)]
pub struct KCCFloorSnap;

/// Component that handles gravity for a kinematic character controller
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct KCCGravity {
    /// The maximum velocity the character can reach when falling
    pub terminal_velocity: f32,
    /// The acceleration factor (9.81 on Earth)
    pub acceleration_factor: f32,
    /// Current velocity from gravity
    pub current_velocity: Vec3,
    /// Direction of gravity
    pub direction: Vec3,
}

impl Default for KCCGravity {
    fn default() -> Self {
        Self {
            terminal_velocity: 53.0, // ~terminal velocity for human
            acceleration_factor: 9.81 * 2.0,
            current_velocity: Vec3::ZERO,
            direction: Vec3::NEG_Y,
        }
    }
}

/// Component that controls how the character handles slopes
#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct KCCSlope {
    /// Maximum angle in radians that the character can walk up
    pub max_slope_angle: f32,
    /// Friction coefficient applied when on slopes
    pub friction: f32,
}

impl Default for KCCSlope {
    fn default() -> Self {
        Self { max_slope_angle: 80.0_f32.to_radians(), friction: 0.8 }
    }
}

/// Function that updates the kinematic character controller's internal state.
/// Currently, this only updates the previous velocity.
pub fn update_kinematic_character_controller(
    mut query: Query<(&mut KinematicCharacterController, &mut LinearVelocity)>,
) {
    for (mut controller, _) in query.iter_mut() {
        controller.prev_velocity = controller.velocity;
        // linear_velocity.0 = controller.velocity;
    }
}

pub fn update_kinematic_floor(
    mut query: Query<(&mut KCCFloorDetection, &Transform, Option<&mut KCCGrounded>, Entity)>,
    spatial_query: SpatialQuery,
) {
    for (mut floor_detection, transform, mut grounded, entity) in query.iter_mut() {
        floor_detection.prev_floor_normal = floor_detection.floor_normal;
        if let Some(grounded) = grounded.as_mut() {
            grounded.prev_grounded = grounded.grounded;
        }

        if let Some(cast) = spatial_query.cast_shape(
            &floor_detection.floor_collider,
            transform.translation,
            Quat::IDENTITY,
            Dir3::new_unchecked(floor_detection.ground_direction.normalize()),
            &ShapeCastConfig { max_distance: floor_detection.max_floor_distance, ..default() },
            &SpatialQueryFilter::default().with_excluded_entities([entity]),
        ) {
            floor_detection.floor_normal = cast.normal1;
            floor_detection.floor_distance = cast.distance;
            if let Some(grounded) = grounded.as_mut() {
                grounded.grounded = true;
            }
        } else {
            if let Some(grounded) = grounded.as_mut() {
                grounded.grounded = false;
            }
        };
    }
}

pub fn floor_snap(
    mut query: Query<(
        &mut Transform,
        &KCCFloorDetection,
        &KCCGrounded,
        Option<&KCCFloorSnap>,
        &KinematicCharacterController,
    )>,
) {
    for (mut transform, floor_detection, grounded, _, controller) in query.iter_mut() {
        if (grounded.grounded || grounded.prev_grounded)
            && controller.velocity.y <= 0.0
            && floor_detection.floor_distance < 0.01
        {
            transform.translation.y -= floor_detection.floor_distance - 0.001;
        }
    }
}

pub fn velocity_dampening(mut query: Query<&mut KinematicCharacterController>, _time: Res<Time>) {
    for mut kcc in query.iter_mut() {
        // don't dampen vertical velocity because we want to choreograph it more.
        kcc.velocity.x *= 0.9;
        kcc.velocity.z *= 0.9;
    }
}

/// Optimized gravity system with terminal velocity handling
pub fn gravity_system(
    mut query: Query<(&KinematicCharacterController, &mut KCCGravity, &KCCGrounded)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (_, mut gravity, grounded) in query.iter_mut() {
        if grounded.grounded {
            gravity.current_velocity = Vec3::ZERO;
        }

        let current_speed = gravity.current_velocity.length();
        if current_speed >= gravity.terminal_velocity {
            // Decelerate to terminal velocity
            gravity.current_velocity *= 0.99;
            continue;
        }

        let delta_velocity = gravity.direction * gravity.acceleration_factor * dt;
        let new_velocity = gravity.current_velocity + delta_velocity;

        gravity.current_velocity = if new_velocity.length() > gravity.terminal_velocity {
            new_velocity.normalize() * gravity.terminal_velocity
        } else {
            new_velocity
        };
    }
}

#[derive(Component, Debug, Reflect)]
pub struct KCCJump {
    pub initial_force: f32, // starting force applied each frame while holding jump
    pub hold_falloff: f32,  // falloff by this amount * delta each frame we hold jump
    pub falloff: f32,       // falloff by this amount * delta each frame

    pub last_jump: bool,
    pub current_force: Option<f32>,
}

impl Default for KCCJump {
    fn default() -> Self {
        Self {
            initial_force: 15.0,
            hold_falloff: 25.0,
            falloff: 75.0,

            last_jump: false,
            current_force: None,
        }
    }
}

pub fn handle_jump(
    mut players: Query<(
        &mut KinematicCharacterController,
        &KCCGrounded,
        &mut KCCJump,
        &Actions<PlayerInput>,
    )>,
    time: Res<Time>,
) -> Result<()> {
    for (mut controller, grounded, mut jump, actions) in &mut players {
        let mut falloff = 0.0;
        match actions.state::<Jump>()? {
            ActionState::Fired => {
                if grounded.grounded {
                    if jump.current_force.is_none() && !jump.last_jump {
                        jump.last_jump = true;
                        jump.current_force = Some(jump.initial_force);
                    } else if jump.current_force.is_some() {
                        jump.current_force = None;
                    }
                } else {
                    falloff = jump.hold_falloff;
                }
            },
            _ => {
                jump.last_jump = false;
                falloff = jump.falloff;
            },
        }

        if let Some(force) = &mut jump.current_force {
            *force -= falloff * time.delta_secs();
        }

        if jump.current_force.is_some_and(|force| force < 0.0) {
            jump.current_force = None;
        }

        if let Some(force) = jump.current_force {
            controller.velocity += Vec3::Y * force;
        }
    }

    Ok(())
}
