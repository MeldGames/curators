use avian3d::prelude::*;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use bevy_mod_outline::OutlineVolume;

pub mod prefab_registry;

#[derive(Component, Clone, Copy, Debug, Reflect)]
// #[require(SweptCcd, SleepingDisabled)]
#[reflect(Component, Clone)]
pub struct Item;

pub fn plugin(app: &mut App) {
    app.add_plugins(prefab_registry::plugin);

    app.add_systems(Startup, spawn_test_items);
    app.add_systems(Update, ItemOutline::lerp_color);
}

pub fn spawn_test_items(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Item,
        Name::new("Test item (sphere)"),
        Mesh3d(meshes.add(Sphere::new(0.3))),
        MeshMaterial3d(
            materials
                .add(StandardMaterial { base_color: Color::WHITE.into(), ..Default::default() }),
        ),
        RigidBody::Dynamic,
        Collider::sphere(0.3),
        Transform::from_xyz(2.0, 7.0, 2.0),
    ));

    commands.spawn((
        Item,
        Name::new("Test item (sphere)"),
        Mesh3d(meshes.add(Sphere::new(0.3))),
        MeshMaterial3d(
            materials
                .add(StandardMaterial { base_color: Color::WHITE.into(), ..Default::default() }),
        ),
        RigidBody::Dynamic,
        Collider::sphere(0.3),
        Transform::from_xyz(3.0, 7.0, 2.0),
    ));
}

#[derive(Component)]
pub struct ItemOutline {
    pub alpha_range: std::ops::RangeInclusive<f32>, // current alpha
    pub step: f32,                                  // step amount per second
    pub direction: bool,                            // true -> up, false -> down
}

impl Default for ItemOutline {
    fn default() -> Self {
        Self { alpha_range: 0.7..=1.0, step: 2.0, direction: true }
    }
}

impl ItemOutline {
    pub fn lerp_color(
        mut outlines: Query<(&mut OutlineVolume, &mut ItemOutline)>,
        time: Res<Time>,
    ) {
        for (mut volume, mut meta) in &mut outlines {
            let mut alpha = volume.colour.alpha();
            let step = meta.step * (meta.alpha_range.end() - meta.alpha_range.start());

            if meta.direction {
                alpha += step * time.delta_secs();
                if alpha > *meta.alpha_range.end() {
                    let over = alpha - *meta.alpha_range.end();
                    alpha = *meta.alpha_range.end() - over;
                    meta.direction = false;
                }
            } else {
                alpha -= step * time.delta_secs();
                if alpha < *meta.alpha_range.start() {
                    let over = alpha - *meta.alpha_range.start();
                    alpha = *meta.alpha_range.start() - over;
                    meta.direction = true;
                }
            }

            volume.colour.set_alpha(alpha);
        }
    }
}
