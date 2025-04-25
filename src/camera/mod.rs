use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

pub mod flying;
pub mod follow;

pub use flying::{FlyingCamera, FlyingSettings, FlyingState};
pub use follow::{FollowCamera, FollowSettings, FollowState};

pub fn plugin(app: &mut App) {
    app.add_plugins(follow::plugin).add_plugins(flying::plugin);

    app.add_input_context::<CameraToggle>();

    app.add_systems(PostUpdate, changed_camera_toggle);
    
    app.add_observer(toggle_binding).add_observer(switch_cameras);
}

#[derive(InputContext)]
#[input_context(priority = 1)]
pub struct CameraToggle;

#[derive(InputAction, Debug)]
#[input_action(output = bool)]
pub struct Toggle;

#[derive(Reflect)]
pub enum ActiveCamera {
    Flying,
    Follow,
}

#[derive(Component, Reflect)]
pub struct CameraEntities {
    pub follow: Entity,
    pub flying: Entity,
    pub active: ActiveCamera,
}

pub fn toggle_binding(trigger: Trigger<Binding<CameraToggle>>, mut toggle: Query<&mut Actions<CameraToggle>>) {
    let Ok(mut actions) = toggle.get_mut(trigger.entity()) else {
        return;
    };

    actions.bind::<Toggle>().to(KeyCode::KeyP.with_conditions(Release::default()));
}

impl CameraEntities {
    pub fn assert_state(&self, commands: &mut Commands, cameras: &mut Query<&mut Camera>) {
        let Ok([mut flying_camera, mut follow_camera]) = cameras.get_many_mut([self.flying, self.follow]) else {
            return;
        };

        match self.active {
            ActiveCamera::Flying => {
                commands.entity(self.follow).remove::<Actions<FollowCamera>>();
                follow_camera.is_active = false;

                commands.entity(self.flying).insert(Actions::<FlyingCamera>::default());
                flying_camera.is_active = true;
            }
            ActiveCamera::Follow => {
                commands.entity(self.flying).remove::<Actions<FlyingCamera>>();
                flying_camera.is_active = false;

                commands.entity(self.follow).insert(Actions::<FollowCamera>::default());
                follow_camera.is_active = true;
            }
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
    let Ok(mut camera_entities) = camera_entities.get_mut(trigger.entity()) else {
        return;
    };

    match camera_entities.active {
        ActiveCamera::Flying => {
            camera_entities.active = ActiveCamera::Follow;
        }
        ActiveCamera::Follow => {
            camera_entities.active = ActiveCamera::Flying;
        }
    }
}
