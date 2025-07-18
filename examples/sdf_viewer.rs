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
    app.run();
}

pub fn voxel_floor(mut voxels: Query<&mut Voxels>, mut initialized: Local<bool>) {
    if *initialized {
        return;
    }

    let Ok(mut voxels) = voxels.single_mut() else {
        return;
    };

    let width = 1000;
    let floor = -10;
    for x in -width..width {
        for z in -width..width {
            voxels.set_voxel(IVec3::new(x, floor, z), Voxel::Base);
        }
    }

    *initialized = true;
}

pub fn rasterize_sdf(mut voxels: Query<&mut Voxels>, input: Res<ButtonInput<KeyCode>>) {
    if !input.just_pressed(KeyCode::KeyP) {
        return;
    }

    let Ok(mut voxels) = voxels.single_mut() else {
        return;
    };

    // let sdf = sdf::Sphere { radius: 5.0 };
    let sdf = sdf::Torus { minor_radius: 5.0, major_radius: 10.0 };
    let sdf = ops::Twist {
        primitive: ops::Isometry {
            primitive: sdf,
            rotation: Quat::from_axis_angle(Vec3::Z, 90.0f32.to_radians()),
            translation: Vec3::ZERO,
        },
        strength: 0.3,
    };

    // let sdf = ops::Union { a: sdf, b: sdf::Sphere { radius: 2.0 } };
    let translated_sphere = ops::Isometry {
        translation: Vec3::new(12.0, 12.0, 3.0),
        rotation: Quat::IDENTITY,
        primitive: sdf::Sphere { radius: 8.0 },
    };

    let sdf = ops::SmoothUnion { a: sdf, b: translated_sphere, k: 5.0 };
    // let sdf = ops::Intersection { a: sdf, b: translated_sphere };
    for raster_voxel in voxel_rasterize::rasterize(
        sdf,
        RasterConfig {
            clip_bounds: Aabb3d { min: Vec3A::splat(-1000.0), max: Vec3A::splat(1000.0) },
            grid_scale: arch::voxel::GRID_SCALE,
            pad_bounds: Vec3::splat(3.0),
        },
    ) {
        if raster_voxel.distance < 0.0 {
            voxels.set_voxel(raster_voxel.point, Voxel::Grass);
        } else if raster_voxel.distance < 3. {
            voxels.set_voxel(raster_voxel.point, Voxel::Water);
        }
    }
}
