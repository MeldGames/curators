//! Control remeshing amounts per frame.

use bevy::camera::primitives::Frustum;
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.register_type::<Remesh>();
    app.insert_resource(Remesh { ..default() });
    app.insert_resource(RemeshCenter::default());

    app.add_systems(First, accumulate_remesh);
    app.add_systems(First, position_remesh_center);
}

#[derive(Resource, Clone, Copy, Debug, Reflect)]
#[reflect(Resource)]
pub struct Remesh {
    // config
    pub surface_net_per_frame: f32,
    pub bgm_per_frame: f32,
    pub collider_per_frame: f32,

    // usage
    pub surface_net: usize,
    pub bgm: usize,
    pub collider: usize,

    // management
    pub surface_net_accumulator: f32,
    pub bgm_accumulator: f32,
    pub collider_accumulator: f32,
}

impl Default for Remesh {
    fn default() -> Self {
        Self {
            surface_net_per_frame: 0.1,
            bgm_per_frame: 64.0,
            collider_per_frame: 0.5,

            surface_net: 0,
            bgm: 0,
            collider: 0,

            surface_net_accumulator: 0.0,
            bgm_accumulator: 0.0,
            collider_accumulator: 0.0,
        }
    }
}

fn this_frame(accumulator: &mut f32) -> usize {
    let this_frame = accumulator.floor() as usize;
    *accumulator = accumulator.fract();
    this_frame
}

pub fn accumulate_remesh(mut remesh: ResMut<Remesh>) {
    remesh.surface_net_accumulator += remesh.surface_net_per_frame;
    remesh.bgm_accumulator += remesh.bgm_per_frame;
    remesh.collider_accumulator += remesh.collider_per_frame;

    remesh.surface_net = this_frame(&mut remesh.surface_net_accumulator);
    remesh.bgm = this_frame(&mut remesh.bgm_accumulator);
    remesh.collider = this_frame(&mut remesh.collider_accumulator);
}

#[derive(Resource, Debug)]
pub struct RemeshCenter {
    pub transform: Transform,
    pub frustum: Frustum,
}

impl Default for RemeshCenter {
    fn default() -> Self {
        Self { transform: Transform::IDENTITY, frustum: Frustum::default() }
    }
}

pub fn position_remesh_center(
    cameras: Query<(Entity, &GlobalTransform, &Camera, &Frustum)>,
    mut remesh_center: ResMut<RemeshCenter>,
) {
    for (entity, transform, camera, frustum) in cameras {
        if camera.is_active {
            remesh_center.transform = transform.compute_transform();
            remesh_center.frustum = frustum.clone();
            return;
        }
    }
}
