use std::sync::Arc;

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use bevy_math::bounding::Aabb3d;

use crate::sdf;
use crate::sdf::voxel_rasterize::RasterConfig;
use crate::voxel::commands::{SetVoxelsSdfParams, VoxelCommandQueue};
use crate::voxel::raycast::VoxelHit;
use crate::voxel::{CursorVoxel, Voxel, VoxelCommand, VoxelSet, Voxels};

pub fn plugin(app: &mut App) {
    app.add_input_context::<VoxelPainter>();

    app.add_systems(Startup, add_voxel_painter);

    app.add_observer(cycle_brush)
        .add_observer(cycle_voxel)
        .add_observer(paint_voxels)
        .add_observer(erase_voxels);
}

#[derive(Component, Default)]
pub struct VoxelPainter {
    pub brushes: Vec<&'static dyn sdf::Sdf>,
    pub brush_index: usize,

    pub voxels: Vec<Voxel>,
    pub voxel_index: usize,
}

impl VoxelPainter {
    pub fn brush(&self) -> &'static dyn sdf::Sdf {
        self.brushes[self.brush_index]
    }

    pub fn voxel(&self) -> Voxel {
        self.voxels[self.voxel_index]
    }
}

#[derive(InputAction, Debug, Default)]
#[action_output(bool)]
pub struct Paint;

#[derive(InputAction, Debug, Default)]
#[action_output(bool)]
pub struct Erase;

#[derive(InputAction, Debug, Default)]
#[action_output(bool)]
pub struct CycleBrush;

#[derive(InputAction, Debug, Default)]
#[action_output(bool)]
pub struct CycleVoxel;

pub fn add_voxel_painter(mut commands: Commands) {
    commands.spawn((
        Name::new("Voxel painter"),
        VoxelPainter {
            brushes: vec![
                &Cuboid { half_size: crate::voxel::GRID_SCALE },
                &sdf::Torus { minor_radius: 4.0, major_radius: 12.0 },
                &sdf::Sphere { radius: 3.0 },
            ],
            brush_index: 0,

            voxels: vec![
                Voxel::Dirt,
                Voxel::Sand,
                // Voxel::Water(default()),
                // Voxel::Oil(default())
            ],
            voxel_index: 0,
        },
        actions![VoxelPainter[
            (Action::<Erase>::new(), bindings![
                (MouseButton::Left, Press::default()),
                Binding::MouseButton { button: MouseButton::Left, mod_keys: ModKeys::SHIFT },
            ]),
            (Action::<Paint>::new(), bindings![
                (MouseButton::Right, Press::default()),
                Binding::MouseButton { button: MouseButton::Right, mod_keys: ModKeys::SHIFT },
            ]),
            (Action::<CycleVoxel>::new(), Press::default(), bindings![KeyCode::KeyV]),
            (Action::<CycleBrush>::new(), Press::default(), bindings![KeyCode::KeyB]),
        ]],
    ));
}

pub fn cycle_voxel(trigger: Trigger<Fired<CycleVoxel>>, mut painters: Query<&mut VoxelPainter>) {
    let Ok(mut painter) = painters.get_mut(trigger.target()) else {
        return;
    };
    painter.voxel_index += 1;
    painter.voxel_index %= painter.voxels.len();
    info!("voxel: {:?}", painter.voxel());
}

pub fn cycle_brush(trigger: Trigger<Fired<CycleBrush>>, mut painters: Query<&mut VoxelPainter>) {
    let Ok(mut painter) = painters.get_mut(trigger.target()) else {
        return;
    };
    painter.brush_index += 1;
    painter.brush_index %= painter.brushes.len();
}

pub fn paint_voxels(
    trigger: Trigger<Fired<Paint>>,
    cursor_voxel: Res<CursorVoxel>,
    painters: Query<&VoxelPainter>,

    mut commands: Query<&mut VoxelCommandQueue>,
) {
    // info!("painting");
    let Ok(painter) = painters.get(trigger.target()) else {
        return;
    };

    if let Some(hit) = cursor_voxel.hit() {
        let normal = hit.normal.unwrap_or(IVec3::Y);
        let point = hit.voxel + normal;

        for mut command_queue in &mut commands {
            command_queue.push(VoxelCommand::SetVoxelsSdf {
                center: point,
                sdf: painter.brush().as_node(),
                voxel: painter.voxel(),
                params: SetVoxelsSdfParams { within: 0.0, can_replace: VoxelSet::AIR },
            });
        }
    }
}

pub fn erase_voxels(
    trigger: Trigger<Fired<Erase>>,
    cursor_voxel: Res<CursorVoxel>,
    painters: Query<&VoxelPainter>,

    mut voxels: Query<&mut Voxels>,
) {
    let Ok(painter) = painters.get(trigger.target()) else {
        return;
    };

    // Calculate if and where the ray is hitting a voxel.
    let Ok(mut voxels) = voxels.single_mut() else {
        info!("No voxels found");
        return;
    };

    if let Some(hit) = cursor_voxel.hit() {
        // Place block
        for raster_voxel in crate::sdf::voxel_rasterize::rasterize(painter.brush(), RasterConfig {
            clip_bounds: Aabb3d { min: Vec3A::splat(-1000.0), max: Vec3A::splat(1000.0) },
            grid_scale: crate::voxel::GRID_SCALE,
            pad_bounds: Vec3::splat(3.0),
        }) {
            let point = hit.voxel + raster_voxel.point;
            if raster_voxel.distance < 0.0 {
                if voxels.get_voxel(point).breakable() {
                    voxels.set_voxel(point, Voxel::Air);
                }
            }
        }
    }
}
