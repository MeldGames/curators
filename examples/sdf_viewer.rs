use arch::core::sdf::voxel_rasterize::RasterConfig;
use arch::core::sdf::{self, Sdf, ops, voxel_rasterize};
use arch::core::voxel::{Voxel, Voxels};
use bevy::prelude::*;
use bevy_math::bounding::Aabb3d;

pub fn main() {
    let mut app = App::new();
    arch::core::viewer(&mut app);
    app.add_plugins(arch::core::voxel::VoxelPlugin);
    app.insert_resource(AmbientLight { brightness: 2500.0, ..default() });
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
        primitive: sdf.rotate(Quat::from_axis_angle(Vec3::Z, 90.0f32.to_radians())),
        strength: 0.3,
    };

    // let sdf = ops::Union { A: Sdf + Clone + Default, B: Sdf + Clone + Default::Sphere { radius: 2.0 } };
    let translated_sphere = sdf::Sphere { radius: 8.0 }.translate(Vec3::new(12.0, 12.0, 3.0));

    let sdf = ops::SmoothUnion { a: sdf, b: translated_sphere, k: 5.0 };
    // let sdf = ops::Intersection { a: sdf, b: translated_sphere };
    for raster_voxel in voxel_rasterize::rasterize(
        sdf,
        RasterConfig {
            clip_bounds: Aabb3d { min: Vec3A::splat(-1000.0), max: Vec3A::splat(1000.0) },
            grid_scale: arch::core::voxel::GRID_SCALE,
            pad_bounds: Vec3::splat(3.0),
        },
    ) {
        if raster_voxel.distance < 0.0 {
            voxels.set_voxel(raster_voxel.point, Voxel::Grass);
        } else if raster_voxel.distance < 3. {
            // voxels.set_voxel(raster_voxel.point, Voxel::Water {
            // lateral_energy: 4 });
        }
    }
}
