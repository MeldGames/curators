use arch_core::voxel::simulation::FallingSandTick;
use bench::falling_sands::{basic_benches, paint_brush};
use bevy::core_pipeline::core_3d::graph::Node3d;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use arch::core::camera::{FlyingCamera, FlyingSettings, FlyingState, camera_components};
use arch::core::sdf::voxel_rasterize::RasterConfig;
use arch::core::sdf::{self, Sdf, ops, voxel_rasterize};
use arch::core::voxel::{Voxel, Voxels};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

pub fn main() {
    let mut app = App::new();
    arch::core::viewer(&mut app);

    app.register_type::<FallingSandTick>();
    app.insert_resource(FallingSandTick(0));
    app.insert_resource(CurrentBench::new(0));
    app.insert_resource(AmbientLight { brightness: 2500.0, ..default() });

    app.add_systems(
        Update,
        (arch::core::voxel::simulation::falling_sands, increment_step).run_if(should_run),
    );
    app.add_systems(PostUpdate, arch::core::voxel::simulation::update_render_voxels);

    app.add_plugins(arch::core::voxel::simulation::data::plugin);

    app.add_plugins(arch::core::voxel::voxel::plugin)
        .add_plugins(arch::core::voxel::mesh::plugin)
        .add_plugins(arch::core::voxel::voxels::plugin);
    // .add_plugins(arch::core::voxel::collider::plugin)
    // .add_plugins(arch::core::voxel::raycast::plugin)

    app.add_systems(Startup, arch::core::voxel::spawn_directional_lights);

    app.add_systems(Update, cycle_benches);
    app.run();
}

#[derive(Resource, Clone, Debug)]
pub struct CurrentBench {
    pub bench_index: usize,
    pub step: usize,
    pub max_steps: usize,
    pub running: bool,
}

impl CurrentBench {
    pub fn new(bench_index: usize) -> Self {
        Self {
            bench_index,
            step: 0,
            max_steps: basic_benches()[bench_index].test_steps,
            running: false,
        }
    }
}

pub fn should_run(input: Res<ButtonInput<KeyCode>>, current_bench: Res<CurrentBench>) -> bool {
    (current_bench.running && current_bench.step < current_bench.max_steps)
        || input.just_pressed(KeyCode::KeyK)
        || (input.pressed(KeyCode::ShiftLeft) && input.pressed(KeyCode::KeyK))
}

pub fn increment_step(mut current_bench: ResMut<CurrentBench>) {
    current_bench.step += 1;
    println!("step {}", current_bench.step);
}

pub fn cycle_benches(
    mut commands: Commands,
    voxels: Query<Entity, With<Voxels>>,
    input: Res<ButtonInput<KeyCode>>,
    mut current_bench: ResMut<CurrentBench>,
) {
    let mut benches = basic_benches();
    if input.just_pressed(KeyCode::KeyN) {
        current_bench.bench_index = (current_bench.bench_index + 1) % benches.len();
    } else if input.just_pressed(KeyCode::KeyB) {
        if current_bench.bench_index > 0 {
            current_bench.bench_index = (current_bench.bench_index - 1) % benches.len();
        } else {
            current_bench.bench_index = benches.len() - 1;
        }
    }

    if input.just_pressed(KeyCode::KeyH) {
        current_bench.running = !current_bench.running;
    }

    if !input.just_pressed(KeyCode::KeyR) {
        return;
    }

    let bench = benches.remove(current_bench.bench_index);
    info!("showing bench {:?}", bench.name);
    current_bench.step = 0;
    current_bench.max_steps = bench.test_steps;

    for entity in voxels.iter() {
        commands.entity(entity).despawn();
    }

    let mut new_voxels = Voxels::new(bench.voxel_size);
    for (center, brush, voxel) in &bench.brushes {
        paint_brush(&mut new_voxels, *center, &**brush, *voxel);
    }

    commands.spawn(new_voxels);
}
