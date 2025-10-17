use bevy::prelude::*;
use bevy_enhanced_input::prelude::{Press, *};

use crate::sdf;
use crate::voxel::commands::SetVoxelsSdfParams;
use crate::voxel::{CursorVoxel, Voxel, VoxelCommand, VoxelSet};

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
                &Cuboid { half_size: Vec3::ONE },
                &sdf::Torus { minor_radius: 4.0, major_radius: 12.0 },
                &sdf::Sphere { radius: 5.0 },
            ],
            brush_index: 0,

            voxels: vec![
                Voxel::Dirt,
                Voxel::Sand,
                Voxel::Water(default()),
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

pub fn cycle_voxel(trigger: On<Fire<CycleVoxel>>, mut painters: Query<&mut VoxelPainter>) {
    let Ok(mut painter) = painters.get_mut(trigger.target()) else {
        return;
    };
    painter.voxel_index += 1;
    painter.voxel_index %= painter.voxels.len();
    info!("voxel: {:?}", painter.voxel());
}

pub fn cycle_brush(trigger: On<Fire<CycleBrush>>, mut painters: Query<&mut VoxelPainter>) {
    let Ok(mut painter) = painters.get_mut(trigger.target()) else {
        return;
    };
    painter.brush_index += 1;
    painter.brush_index %= painter.brushes.len();
}

pub fn paint_voxels(
    trigger: On<Fire<Paint>>,
    cursor_voxel: Res<CursorVoxel>,
    painters: Query<&VoxelPainter>,

    mut commands: EventWriter<VoxelCommand>,
) {
    // info!("painting");
    let Ok(painter) = painters.get(trigger.target()) else {
        return;
    };

    if let Some(hit) = cursor_voxel.hit() {
        let normal = hit.normal.unwrap_or(IVec3::Y);
        let point = hit.voxel + normal;

        // info!("painting at {:?}", hit);
        commands.write(VoxelCommand::SetVoxelsSdf {
            origin: point,
            sdf: painter.brush().as_node(),
            voxel: painter.voxel(),
            params: SetVoxelsSdfParams { within: 0.0, can_replace: VoxelSet::AIR },
        });
    }
}

pub fn erase_voxels(
    trigger: On<Fire<Erase>>,
    cursor_voxel: Res<CursorVoxel>,
    painters: Query<&VoxelPainter>,

    mut commands: MessageWriter<VoxelCommand>,
) {
    let Ok(painter) = painters.get(trigger.target()) else {
        return;
    };

    if let Some(hit) = cursor_voxel.hit() {
        let normal = hit.normal.unwrap_or(IVec3::Y);
        let point = hit.voxel;

        info!("erasing at {:?}", hit);
        commands.write(VoxelCommand::SetVoxelsSdf {
            origin: point,
            sdf: painter.brush().as_node(),
            voxel: Voxel::Air,
            params: SetVoxelsSdfParams { within: 0.0, can_replace: VoxelSet::BREAKABLE },
        });
    }
}
