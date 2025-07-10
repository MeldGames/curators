use bevy_math::bounding::Aabb3d;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use bevy::prelude::*;

use arch::sdf::{
    self,
    voxel_rasterize::{RasterConfig, RasterVoxel, rasterize},
};
use arch::voxel::{self, Voxel, Voxels};

criterion_group!(benches, falling_sand_torus);
criterion_main!(benches);

fn falling_sand_torus(c: &mut Criterion) {
    let mut group = c.benchmark_group("falling_sand");

    group.bench_function("sequential_systems", |b| {
        let mut voxels = Voxels::new();

        let torus = sdf::Torus { minor_radius: 2.0, major_radius: 3.0 };
        for raster_voxel in rasterize(
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

        let floor = -10;
        for x in -15..15 {
            for z in -15..15 {
                voxels.set_voxel(IVec3::new(x, floor, z), Voxel::Base);
            }
        }

        b.iter_batched(
            || {
                let mut app = plugin_setup();
                app.world_mut().spawn((voxels.clone(),));
                app
            },
            |mut app: App| app.update(),
            BatchSize::LargeInput,
        );
    });
}

fn plugin_setup() -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            // Render plugins
            .disable::<bevy::winit::WinitPlugin>()
            .disable::<bevy::window::WindowPlugin>()
            .disable::<bevy::render::RenderPlugin>()
            .disable::<bevy::render::texture::ImagePlugin>()
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>()
            .disable::<bevy::core_pipeline::CorePipelinePlugin>()
            .disable::<bevy::sprite::SpritePlugin>()
            .disable::<bevy::text::TextPlugin>()
            .disable::<bevy::ui::UiPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::picking::PickingPlugin>()
            .disable::<bevy::picking::InteractionPlugin>()
            .disable::<bevy::picking::input::PointerInputPlugin>()
            .disable::<bevy::pbr::PbrPlugin>(),
    );
    // .add_plugins(bevy::app::ScheduleRunnerPlugin::run_loop(std::time::Duration::from_secs_f64(
    //     1.0 / 60.0 as f64,
    // )))
    // .add_plugins(voxel::voxels::plugin)
    // .add_plugins(voxel::simulation::plugin);
    app
}
