use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

pub mod digsite;
pub mod flying;
pub mod follow;

pub use digsite::{DigsiteCamera, DigsiteEntity, DigsiteSettings, DigsiteState};
pub use flying::{FlyingCamera, FlyingSettings, FlyingState};
pub use follow::{FollowCamera, FollowPlayer, FollowSettings, FollowState};

pub fn plugin(app: &mut App) {
    app.register_type::<ActiveCamera>();
    app.add_input_context::<CameraToggle>();

    app.add_plugins(follow::plugin).add_plugins(flying::plugin).add_plugins(digsite::plugin);
    app.add_systems(Update, changed_camera_toggle);
    app.add_observer(toggle_binding).add_observer(switch_cameras);
}

#[derive(InputContext)]
#[input_context(priority = 1)]
pub struct CameraToggle;

#[derive(InputAction, Debug)]
#[input_action(output = bool)]
pub struct Toggle;

#[derive(Reflect, Default)]
pub enum ActiveCamera {
    Flying,
    Follow,
    #[default]
    Digsite,
}

#[derive(Component, Reflect)]
pub struct CameraEntities {
    pub follow: Entity,
    pub flying: Entity,
    pub digsite: Entity,
    pub active: ActiveCamera,
}

pub fn toggle_binding(
    trigger: Trigger<Binding<CameraToggle>>,
    mut toggle: Query<&mut Actions<CameraToggle>>,
) {
    let Ok(mut actions) = toggle.get_mut(trigger.target()) else {
        return;
    };

    actions.bind::<Toggle>().to(KeyCode::KeyP.with_conditions(Release::default()));
}

impl CameraEntities {
    pub fn assert_state(&self, commands: &mut Commands, cameras: &mut Query<&mut Camera>) {
        let Ok(mut cameras) = cameras.get_many_mut([self.flying, self.follow, self.digsite]) else {
            return;
        };

        for camera in &mut cameras {
            if camera.is_active {
                camera.is_active = false;
            }
        }

        let [mut flying_camera, mut follow_camera, mut digsite_camera] = cameras;

        match self.active {
            ActiveCamera::Flying => {
                flying_camera.is_active = true;
            },
            ActiveCamera::Follow => {
                follow_camera.is_active = true;
            },
            ActiveCamera::Digsite => {
                digsite_camera.is_active = true;
            },
        }

        if follow_camera.is_active {
            commands.entity(self.follow).insert_if_new(Actions::<FollowCamera>::default());
        } else {
            commands.entity(self.follow).remove::<Actions<FollowCamera>>();
        }

        if flying_camera.is_active {
            commands.entity(self.flying).insert_if_new(Actions::<FlyingCamera>::default());
        } else {
            commands.entity(self.flying).remove::<Actions<FlyingCamera>>();
        }

        if digsite_camera.is_active {
            commands.entity(self.digsite).insert_if_new(Actions::<DigsiteCamera>::default());
        } else {
            commands.entity(self.digsite).remove::<Actions<DigsiteCamera>>();
        }
    }
}

pub fn changed_camera_toggle(
    camera_entities: Query<&CameraEntities, Changed<CameraEntities>>,
    mut commands: Commands,
    mut cameras: Query<&mut Camera>,
) {
    for camera_entity in &camera_entities {
        camera_entity.assert_state(&mut commands, &mut cameras);
    }
}

pub fn switch_cameras(
    trigger: Trigger<Fired<Toggle>>,
    mut camera_entities: Query<&mut CameraEntities>,
) {
    let Ok(mut camera_entities) = camera_entities.get_mut(trigger.target()) else {
        return;
    };

    match camera_entities.active {
        ActiveCamera::Flying => {
            camera_entities.active = ActiveCamera::Follow;
        },
        ActiveCamera::Follow => {
            camera_entities.active = ActiveCamera::Digsite;
        },
        ActiveCamera::Digsite => {
            camera_entities.active = ActiveCamera::Flying;
        },
    }
}
