use arch_core::bevy;
use arch_core::bevy::math::bounding::Aabb3d;
use arch_core::sdf::{self, Sdf};
use arch_core::voxel::{self, GRID_SCALE, Voxel, Voxels};
use bevy::prelude::*;

pub fn plugin_setup() -> App {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugins(voxel::voxels::plugin)
        .insert_resource(voxel::simulation::FallingSandTick(0))
        .add_systems(Update, voxel::simulation::falling_sands)
        .add_plugins(voxel::simulation::data::plugin);
    app
}

pub fn paint_brush(voxels: &mut Voxels, center: IVec3, brush: &dyn Sdf, voxel: Voxel) {
    for raster_voxel in sdf::voxel_rasterize::rasterize(
        brush,
        sdf::voxel_rasterize::RasterConfig {
            clip_bounds: Aabb3d::new(Vec3::ZERO, Vec3::splat(1000.0)),
            grid_scale: GRID_SCALE,
            pad_bounds: Vec3::ZERO,
        },
    ) {
        if raster_voxel.distance <= 0.0 {
            voxels.set_voxel(raster_voxel.point + center, voxel);
        }
    }
}

pub trait SdfSendSync: Sdf + Send + Sync + 'static {}
impl<T: Sdf + Send + Sync + 'static> SdfSendSync for T {}

#[derive(Clone)]
pub struct BenchSetup {
    /// Name of the bench setup
    pub name: &'static str,
    /// Size of the voxel grid
    pub voxel_size: IVec3,
    /// How many update iterations to run
    pub test_steps: usize,
    /// Paint voxels in the world each step: (center, brush, voxel)
    pub brushes: Vec<(IVec3, &'static (dyn Sdf + Send + Sync), Voxel)>,
}

lazy_static::lazy_static! {
    pub static ref BASIC_BENCHES: Vec<BenchSetup> =
        [
            BenchSetup {
                name: "torus_sand",
                voxel_size: IVec3::new(60, 60, 60),
                test_steps: 40,
                brushes: vec![(
                    IVec3::new(30, 30, 30),
                    &sdf::Torus { minor_radius: 2.0, major_radius: 3.0 },
                    Voxel::Sand,
                )],
            },
            BenchSetup {
                name: "torus_water",
                voxel_size: IVec3::new(60, 60, 60),
                test_steps: 40,
                brushes: vec![(
                    IVec3::new(30, 30, 30),
                    &sdf::Torus { minor_radius: 2.0, major_radius: 3.0 },
                    Voxel::Water { lateral_energy: 32 },
                )],
            },
        ].to_vec();
}
