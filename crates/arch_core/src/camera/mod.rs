use bevy::core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass};
use bevy::anti_alias::smaa::{Smaa, SmaaPreset};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::SunDisk;
use bevy::pbr::{Atmosphere, AtmosphereMode, AtmosphereSettings, ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel};
use bevy::prelude::*;
// use bevy_edge_detection::EdgeDetection;
use bevy_enhanced_input::prelude::*;

// pub mod digsite;
pub mod flying;
// pub mod follow;

// pub use digsite::{DigsiteCamera, DigsiteEntity, DigsiteSettings,
// DigsiteState};
pub use flying::{FlyingCamera, FlyingSettings, FlyingState};

// pub use follow::{FollowCamera, FollowPlayer, FollowSettings, FollowState};
use crate::voxel::mesh::camera_inside::BlockingMeshes;

pub fn plugin(app: &mut App) {
    app.register_type::<ActiveCamera>();
    app.add_input_context::<CameraToggle>();

    // app.add_plugins(follow::plugin).add_plugins(flying::plugin).
    // add_plugins(digsite::plugin);
    app.add_plugins(flying::plugin);
    app.add_systems(Update, changed_camera_toggle);
    app.add_observer(switch_cameras);
}

pub fn camera_components() -> impl Bundle {
    (
        Camera { ..default() },
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection::default()),
        // This will write the depth buffer to a texture that you can use in the main pass
        DepthPrepass,
        // This will generate a texture containing world normals (with normal maps applied)
        NormalPrepass,
        // This will generate a texture containing screen space pixel motion vectors
        MotionVectorPrepass,
        Tonemapping::default(),
        Atmosphere::EARTH,
        AtmosphereSettings { 
            rendering_method: AtmosphereMode::Raymarched, 
            ..default() 
        },
        SunDisk::EARTH,


        // Exposure::SUNLIGHT,
        // Bloom::NATURAL,
        /*bevy::core_pipeline::auto_exposure::AutoExposure {
            range: -3.0..=3.0,
            // range: -9.0..=1.0,
            filter: 0.10..=0.90,
            speed_brighten: 3.0, // 3.0 default
            speed_darken: 1.0,   // 1.0 default
            // metering_mask: metering_mask.clone(),
            ..default()
        },*/
        // ShadowFilteringMethod::Temporal,
        Msaa::Off,
        // TemporalAntiAliasing::default(),
        ScreenSpaceAmbientOcclusion {
            quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
            constant_object_thickness: 4.0,
        },
        // EdgeDetection {
        //     depth_threshold: 0.3,
        //     normal_threshold: 0.7,
        //     depth_thickness: 1.0,
        //     edge_color: Color::srgba(0.0, 0.0, 0.0, 0.5),
        //     enable_depth: true,
        //     enable_normal: true,
        //     enable_color: false,

        //     uv_distortion_frequency: Vec2::new(1.0, 1.0),
        //     uv_distortion_strength: Vec2::new(0.0, 0.0),
        //     ..default()
        // },
        Smaa { preset: SmaaPreset::Ultra },
        // Fxaa::default(),
        BlockingMeshes { per_x: 40, per_y: 40, mesh_entities: Vec::new() },
    )
}

#[derive(Component)]
pub struct CameraToggle;

#[derive(InputAction, Debug)]
#[action_output(bool)]
pub struct Toggle;

#[derive(Reflect, Default)]
pub enum ActiveCamera {
    #[default]
    Flying,
    Player,
    // Digsite,
}

#[derive(Component, Reflect)]
pub struct CameraEntities {
    // pub follow: Entity,
    pub player: Entity,
    pub flying: Entity,
    // pub digsite: Entity,
    pub active: ActiveCamera,
}

impl CameraEntities {
    pub fn assert_state(&self, commands: &mut Commands, cameras: &mut Query<&mut Camera>) {
        let Ok(mut cameras) = cameras.get_many_mut([self.flying, self.player]) else {
            return;
        };

        for camera in &mut cameras {
            if camera.is_active {
                camera.is_active = false;
            }
        }

        let [mut flying_camera, mut player_camera] = cameras;

        match self.active {
            ActiveCamera::Flying => {
                flying_camera.is_active = true;
            },
            ActiveCamera::Player => {
                player_camera.is_active = true;
            },
            // ActiveCamera::Digsite => {
            //     digsite_camera.is_active = true;
            // },
        }

        // if player_camera.is_active {
        //     commands.entity(self.player).insert(ContextActivity::<FirstPersonCamera>::ACTIVE);
        // } else {
        //     commands.entity(self.player).insert(ContextActivity::<FirstPersonCamera>::INACTIVE);
        // }

        if flying_camera.is_active {
            commands.entity(self.flying).insert(ContextActivity::<FlyingCamera>::ACTIVE);
        } else {
            commands.entity(self.flying).insert(ContextActivity::<FlyingCamera>::INACTIVE);
        }

        // if digsite_camera.is_active {
        //     commands.entity(self.digsite).
        // insert_if_new(Actions::<DigsiteCamera>::default()); } else {
        //     commands.entity(self.digsite).remove::<Actions<DigsiteCamera>>();
        // }
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
            camera_entities.active = ActiveCamera::Player;
        },
        ActiveCamera::Player => {
            // camera_entities.active = ActiveCamera::Digsite;
            camera_entities.active = ActiveCamera::Flying;
        },
        // ActiveCamera::Digsite => {
        //     camera_entities.active = ActiveCamera::Flying;
        // },
    }
}
