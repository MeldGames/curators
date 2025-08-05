//! Try to minimize looking inside voxels by doing some silliness on the near plane.

use bevy::{prelude::*, render::view::NoFrustumCulling};

use crate::voxel::{Voxel, Voxels};

pub fn plugin(app: &mut App) {
    app.register_type::<BlockingMeshes>();
    app.add_observer(added_blocking_meshes);
    app.add_systems(Update, inside_voxel);
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct BlockingMeshes {
    pub per_x: usize,
    pub per_y: usize,
    pub mesh_entities: Vec<Entity>,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct InsideVoxel {
    pub camera_entity: Entity,
    pub current_voxel: Voxel,
}

pub fn added_blocking_meshes(
    trigger: Trigger<OnAdd, BlockingMeshes>,
    mut commands: Commands,
    mut blocking_meshes: Query<&mut BlockingMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("added");
    let Ok(mut blocking) = blocking_meshes.get_mut(trigger.target()) else {
        return;
    };

    for y in 0..blocking.per_y {
        for x in 0..blocking.per_x {
            let pos_x = (x as f32 / blocking.per_x as f32) - 0.5;
            let pos_y = (y as f32 / blocking.per_y as f32) - 0.5;
            blocking.mesh_entities.push(
                commands
                    .spawn((
                        Name::new("Blocking mesh"),
                        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.001)))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.0, 0.0, 1.0),
                            ..default()
                        })),
                        Transform {
                            translation: Vec3::new(pos_x * 0.1, pos_y * 0.2, -0.1),
                            ..default()
                        },
                        ChildOf(trigger.target()),
                        InsideVoxel { camera_entity: trigger.target(), current_voxel: Voxel::Air },
                        Visibility::Hidden,
                        NoFrustumCulling,
                    ))
                    .id(),
            );
        }
    }
}

pub fn inside_voxel(
    grids: Query<(&GlobalTransform, &Voxels)>,
    mut blocking: Query<(
        &GlobalTransform,
        &mut InsideVoxel,
        &mut Visibility,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera: Query<&Camera>,
) {
    for (block_transform, mut inside_voxel, mut visibility, mut material) in &mut blocking {
        let Ok(camera) = camera.get(inside_voxel.camera_entity) else {
            continue;
        };
        if !camera.is_active {
            continue;
        };

        for (grid_transform, voxels) in grids {
            let local_point = grid_transform
                .compute_matrix()
                .inverse()
                .transform_point(block_transform.translation());

            // info!("local_point: {:?}", local_point);
            // info!("world_point: {:?}", block_transform.translation());
            let voxel_point = local_point.as_ivec3();
            let found_voxel = voxels.get_voxel(voxel_point);
            // info!("in voxel: {:?}", found_voxel);
            if inside_voxel.current_voxel != found_voxel {
                inside_voxel.current_voxel = found_voxel;

                match inside_voxel.current_voxel {
                    Voxel::Air | Voxel::Barrier => {
                        *visibility = Visibility::Hidden;
                    },
                    voxel => {
                        *visibility = Visibility::Inherited;
                        *material = MeshMaterial3d(materials.add(voxel.material()));
                    },
                }
            }
        }
    }
}
