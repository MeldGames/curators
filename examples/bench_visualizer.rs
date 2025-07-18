use bevy::core_pipeline::core_3d::graph::Node3d;
use bevy::prelude::*;
use bevy_edge_detection::EdgeDetectionPlugin;
use bevy_enhanced_input::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use arch::camera::{FlyingCamera, FlyingSettings, FlyingState, camera_components};
use arch::sdf::voxel_rasterize::RasterConfig;
use arch::sdf::{self, Sdf, ops, voxel_rasterize};
use arch::voxel::{Voxel, Voxels};
use bevy_math::bounding::Aabb3d;

pub fn main() {
    let mut app = App::new();
    arch::viewer(&mut app);
    app.add_systems(Update, torus_falling);
    app.run();
}

pub fn torus_falling(mut voxels: Query<&mut Voxels>, input: Res<ButtonInput<KeyCode>>) {
    if !input.just_pressed(KeyCode::Space) {
        return;
    }

    info!("spawning new torus");

    let mut voxels = voxels.single_mut().unwrap();
    *voxels = Voxels::new();

    // Create a simulation area with a barrier around it.
    let min = -30;
    let max = 30;
    for x in min..=max {
        for z in min..=max {
            for y in min..=max {
                if x == min || x == max || z == min || z == max || y == min || y == max {
                    voxels.set_voxel(IVec3::new(x, y, z), Voxel::Barrier);
                }
            }
        }
    }

    let torus = sdf::Torus { minor_radius: 2.0, major_radius: 3.0 };
    for raster_voxel in arch::sdf::voxel_rasterize::rasterize(
        torus,
        RasterConfig {
            clip_bounds: Aabb3d::new(Vec3::ZERO, Vec3::splat(100.0)),
            grid_scale: arch::voxel::GRID_SCALE,
            pad_bounds: Vec3::ZERO,
        },
    ) {
        if raster_voxel.distance <= 0.0 {
            voxels.set_voxel(raster_voxel.point, Voxel::Sand);
        }
    }
}
