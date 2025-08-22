use avian_rerecast::prelude::*;
use avian3d::prelude::*;
use bevy::color::palettes::tailwind;
use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;
// use bevy::remote::RemotePlugin;
// use bevy::remote::http::RemoteHttpPlugin;
use bevy_rerecast::debug::DetailNavmeshGizmo;
use bevy_rerecast::prelude::*;

use crate::voxel::mesh::ChangedChunk;

pub fn plugin(app: &mut App) {
    app.add_plugins((NavmeshPlugins::default(), AvianBackendPlugin::default()));

    app.add_systems(FixedPostUpdate, generate_navmesh);
}

#[derive(Resource)]
#[allow(dead_code)]
struct NavmeshHandle(Handle<Navmesh>);

fn generate_navmesh(
    mut generator: NavmeshGenerator,
    handle: Option<ResMut<NavmeshHandle>>,

    mut commands: Commands,
    mut chunk_changed: EventReader<ChangedChunk>,
) {
    if chunk_changed.len() == 0 {
        return;
    }
    chunk_changed.clear();

    let settings = NavmeshSettings { walkable_slope_angle: 85.0f32.to_radians(), ..default() };
    if let Some(handle) = handle {
        generator.regenerate(&handle.0, settings);
    } else {
        let navmesh = generator.generate(settings);
        commands.spawn(DetailNavmeshGizmo::new(&navmesh));
        commands.insert_resource(NavmeshHandle(navmesh));
    }
}
