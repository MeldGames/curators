//! Generate a mesh of a fence given a line and voxel heights

use bevy::prelude::*;
use bevy_turborand::prelude::*;

use crate::map::WorldGenSet;
use crate::voxel::pick::CursorVoxel;
use crate::voxel::{GRID_SCALE, Voxel, Voxels};

pub fn plugin(mut app: &mut App) {
    app.register_type::<Fence>().register_type::<BoardParams>();

    app.add_systems(PreUpdate, spawn_fence.in_set(WorldGenSet::SurfaceDetails));
    app.add_systems(PreUpdate, update_board);

    app.add_systems(PreUpdate, paint_fence);
    app.add_systems(PreUpdate, test_fence);
}

#[derive(Component, Debug, Reflect)]
#[require(Transform, Visibility, Name::new("Fence"))]
#[reflect(Component)]
pub struct Fence {
    pub points: Vec<Vec3>,

    /// Should we connect the last point to the first?
    pub enclosed: bool,

    /// Fence offset from points.
    pub offset: Vec3,

    pub post_size: Vec3,
    pub post_size_variance: Vec3,

    pub boards: Vec<BoardParams>,
}

impl Fence {
    pub fn wooden(points: Vec<Vec3>) -> Self {
        Fence {
            points,

            enclosed: false,
            offset: Vec3::new(0.0, 0.0, 0.0),
            post_size: Vec3::new(0.25, 1.25, 0.25),
            post_size_variance: Vec3::new(0.05, 0.05, 0.0),
            boards: vec![
                BoardParams { offset: Vec3::new(0.0, 0.2, 0.0), ..BoardParams::wooden() },
                BoardParams { offset: Vec3::new(0.0, -0.2, 0.0), ..BoardParams::wooden() },
            ],
        }
    }

    pub fn conform_points_to_voxels(&mut self, voxels: &Voxels) {
        for point in &mut self.points {
            while point.y < 64. * GRID_SCALE.y {
                let voxel_space = (*point / GRID_SCALE).as_ivec3();
                let voxel = voxels.get_voxel(voxel_space);
                if let Some(voxel) = voxel {
                    if !voxel.pickable() {
                        // info!("pickable voxel here? {:?}", voxel_space);
                        break;
                    }
                } else {
                    // info!("no voxel here? {:?}", voxel_space);
                    break;
                }

                point.y += 1. * GRID_SCALE.y;
            }
        }
    }
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct BoardParams {
    pub offset: Vec3,
    pub connection_point_variance: Vec3,
    pub size: Vec3,
    pub size_variance: Vec3,
    pub materials: Vec<StandardMaterial>,
}

impl BoardParams {
    pub fn wooden() -> Self {
        Self {
            offset: Vec3::new(0.0, 0.25, 0.0),
            connection_point_variance: Vec3::new(0.025, 0.05, 0.0),
            size: Vec3::new(0.15, 0.25, 0.0),
            size_variance: Vec3::new(0.025, 0.025, 0.0),
            materials: vec![StandardMaterial {
                base_color: Color::srgb(205. / 255., 157. / 255., 111. / 255.),
                perceptual_roughness: 1.0,
                ..default()
            }],
        }
    }
}

#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub struct Post;

#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub struct Board {
    pub from_post: Entity,
    pub to_post: Entity,
}

pub fn paint_fence(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    mut fence: Local<Option<Entity>>,
    mut fences: Query<&mut Fence>,

    voxels: Query<&Voxels>,
    cursor_voxel: Res<CursorVoxel>,
) {
    let fence = if let Some(fence) = *fence {
        fence
    } else {
        let new_fence = commands.spawn(Fence::wooden(Vec::new())).id();
        *fence = Some(new_fence);
        new_fence
    };

    let Ok((voxels)) = voxels.single() else { return };
    let Ok((mut fence)) = fences.get_mut(fence) else { return };

    if input.just_pressed(KeyCode::KeyO) {
        if let Some(hit) = cursor_voxel.hit() {
            fence.points.push(hit.world_space);
            fence.conform_points_to_voxels(&voxels);
        }
    } else if input.just_pressed(KeyCode::KeyK) {
        fence.points.clear();
    }
}

pub fn test_fence(mut commands: Commands, voxels: Query<&Voxels>, mut done: Local<bool>) {
    if *done {
        return;
    }

    let Ok(voxels) = voxels.get_single() else {
        return;
    };

    let mut points = vec![
        Vec3::new(3.0, 0.0, 0.0),
        // Vec3::new(4.0, 0.0, 0.0),
        Vec3::new(5.0, 0.0, 0.0),
        // Vec3::new(6.0, 0.0, 0.0),
        Vec3::new(7.0, 0.0, 0.0),
        // Vec3::new(7.0, 0.0, 1.0),
        Vec3::new(7.0, 0.0, 2.0),
        Vec3::new(7.0, 0.0, 3.0),
        // Vec3::new(7.0, 0.0, 4.0),
        Vec3::new(7.0, 0.0, 5.0),
        Vec3::new(7.0, 0.0, 6.0),
    ];

    let mut fence = Fence::wooden(points.clone());
    fence.conform_points_to_voxels(&voxels);
    commands.spawn(fence);

    // commands.spawn(Fence::wooden(points));
    *done = true;
}

fn rotation_from_to(start: Vec3, end: Vec3, up: Vec3) -> Quat {
    let direction = (end - start).normalize();

    let fwd = -direction.normalize(); // because local forward is -Z
    let right = up.cross(fwd).normalize();
    let up_corrected = fwd.cross(right);

    // 3x3 rotation matrix: [right, up, forward]
    let rot_matrix = Mat3::from_cols(right, up_corrected, fwd);
    Quat::from_mat3(&rot_matrix)
}

pub fn update_board(
    mut global_rng: ResMut<GlobalRng>,
    mut boards: Query<(Entity, &Board, &BoardParams), Changed<Board>>,
    mut transforms: Query<&mut Transform>,
) {
    for (board_entity, board, board_params) in &mut boards {
        let Ok([mut board_transform, from, to]) =
            transforms.get_many_mut([board_entity, board.from_post, board.to_post])
        else {
            continue;
        };

        let from = from.translation
            + board_params.connection_point_variance
                * Vec3::new(
                    global_rng.f32_normalized(),
                    global_rng.f32_normalized(),
                    global_rng.f32_normalized(),
                );
        let to = to.translation
            + board_params.connection_point_variance
                * Vec3::new(
                    global_rng.f32_normalized(),
                    global_rng.f32_normalized(),
                    global_rng.f32_normalized(),
                );

        let size = board_params.size
            + board_params.size_variance
                * Vec3::new(
                    global_rng.f32_normalized(),
                    global_rng.f32_normalized(),
                    global_rng.f32_normalized(),
                );

        let distance = from.distance(to);
        let midpoint = from.midpoint(to);

        *board_transform = Transform {
            translation: midpoint + board_params.offset,
            rotation: rotation_from_to(from, to, Vec3::Y),
            scale: Vec3::new(size.x, size.y, distance),
        };
    }
}

pub fn spawn_fence(
    mut commands: Commands,
    mut global_rng: ResMut<GlobalRng>,
    fences: Query<(Entity, &Fence, Option<&Children>), Changed<Fence>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (fence_entity, fence, children) in fences {
        if let Some(children) = children {
            for child in children {
                commands.entity(*child).despawn();
            }
        }

        let mut posts = Vec::new();
        for point_index in 0..fence.points.len() {
            let prev_point_index =
                if point_index == 0 { fence.points.len() - 1 } else { point_index - 1 };
            let next_point_index =
                if point_index == fence.points.len() - 1 { 0 } else { point_index + 1 };

            let prev_point = fence.points[prev_point_index];
            let point = fence.points[point_index];
            let next_point = fence.points[next_point_index];

            let post_size = fence.post_size
                + fence.post_size_variance
                    * Vec3::new(
                        global_rng.f32_normalized(),
                        global_rng.f32_normalized(),
                        global_rng.f32_normalized(),
                    );
            let transform = Transform {
                translation: point + Vec3::Y * 0.5 + fence.offset,
                rotation: rotation_from_to(prev_point, next_point, Vec3::Y),
                ..default()
            };

            let post = commands
                .spawn((
                    Post,
                    transform,
                    Mesh3d(meshes.add(Mesh::from(Cuboid::new(
                        post_size.x,
                        post_size.y,
                        post_size.z,
                    )))),
                    MeshMaterial3d(
                        materials.add(StandardMaterial {
                            perceptual_roughness: 1.0,
                            base_color: Color::srgb(205. / 255., 157. / 255., 111. / 255.)
                                .darker(global_rng.f32() * 0.2),
                            ..default()
                        }),
                    ),
                    ChildOf(fence_entity),
                    Name::new("Post"),
                ))
                .id();

            posts.push(post);
        }

        let mut last = None;
        for index in 0..posts.len() {
            let post = posts[index];

            let to_connect = if let Some(last) = last {
                Some((last, post))
            } else {
                if fence.enclosed { Some((posts[posts.len() - 1], post)) } else { None }
            };

            if let Some((from, to)) = to_connect {
                for board_params in &fence.boards {
                    let board = commands
                        .spawn((
                            Board { from_post: from, to_post: to },
                            board_params.clone(),
                            Transform::default(),
                            Mesh3d(meshes.add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)))),
                            MeshMaterial3d(
                                materials.add(StandardMaterial {
                                    base_color: board_params.materials[0]
                                        .base_color
                                        .darker(global_rng.f32() * 0.2),
                                    ..board_params.materials[0].clone()
                                }),
                            ),
                            ChildOf(fence_entity),
                            Name::new("Board"),
                        ))
                        .id();
                }
            }

            last = Some(post);
        }
    }
}
