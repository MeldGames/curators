//! An example that showcases how to update the mesh.
#[allow(unused_imports, dead_code)]
use bevy::pbr::wireframe::{Wireframe, WireframeConfig, WireframePlugin};
use bevy::prelude::*;
use bevy::{color::palettes::css::LIMEGREEN, utils::HashMap};
use bevy_meshem::prelude::*;

use super::voxel_grid::{Voxel, VoxelGrid};

impl Voxel {
    pub fn box_mesh(&self) -> Option<Mesh> {
        match self {
            Voxel::Air => None,
            _ => Some(generate_voxel_mesh(
                [1.0, 1.0, 1.0],
                [1, 4],
                [
                    (Top, [0, 0]),
                    (Bottom, [0, 0]),
                    (Right, [0, 0]),
                    (Left, [0, 0]),
                    (Back, [0, 0]),
                    (Forward, [0, 0]),
                ],
                [0.0, 0.0, 0.0],
                0.05,
                Some(0.8),
                1.0,
            )),
        }
    }
}

pub struct BoxMeshPlugin;
impl Plugin for BoxMeshPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BlockRegistry::new())
            .insert_resource(AmbientLight { brightness: 400.0, color: Color::WHITE });

        app.add_systems(Update, (setup_meshem, toggle_wireframe, meshem_update));

        app.add_event::<ToggleWireframe>().add_event::<RegenerateMesh>();
    }
}

#[derive(Component)]
pub struct MeshemData {
    data: MeshMD<Voxel>,
}

#[derive(Component)]
struct MeshInfo;

#[derive(Component)]
pub struct Meshem;

#[derive(Event, Default)]
struct ToggleWireframe;

#[derive(Event, Default)]
struct RegenerateMesh;

/// Setting up everything to showcase the mesh.
pub fn setup_meshem(
    breg: Res<BlockRegistry>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    grids: Query<(Entity, &VoxelGrid), (Added<VoxelGrid>, With<Meshem>)>,
    // wireframe_config: ResMut<WireframeConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
) {
    for (grid_entity, grid) in &grids {
        let dims: Dimensions = {
            let array = grid.array();
            (array[0] as usize, array[1] as usize, array[2] as usize)
        };
        info!("Setting up meshem for grid: {:?}", dims);
        //let texture_mesh = asset_server.load("array_texture.png");

        let (culled_mesh, metadata) =
            mesh_grid::<Voxel>(dims, &[], &grid.voxels, &*breg, MeshingAlgorithm::Culling, Some(SmoothLightingParameters {
                smoothing: 1.0,
                apply_at_gen: true,
                intensity: 0.5,
                max: 0.6,
            }))
                .unwrap();
        let culled_mesh_handle: Handle<Mesh> = meshes.add(culled_mesh.clone());

        commands.entity(grid_entity)
            .insert((
                Transform {
                    scale: Vec3::new(1.0, 0.2, 1.0),
                    ..default()
                },
                MeshemData { data: metadata },
            ))
            .with_child((
                Transform {
                    translation: Vec3::new(0.5, 0.5, 0.5),
                    ..default()
                },
                Mesh3d(culled_mesh_handle),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::Srgba(LIMEGREEN),
                    //base_color_texture: Some(texture_mesh),
                    ..default()
                })),
            ));
    }
}

#[derive(Resource)]
pub struct BlockRegistry {
    meshes: HashMap<Voxel, Mesh>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        let mut registry = Self { meshes: HashMap::new() };
        registry.add(Voxel::Dirt);
        registry.add(Voxel::Stone);
        registry
    }

    pub fn add(&mut self, voxel: Voxel) {
        if let Some(mesh) = voxel.box_mesh() {
            self.meshes.insert(voxel, mesh);
        }
    }

    pub fn get(&self, voxel: &Voxel) -> Option<&Mesh> {
        self.meshes.get(voxel)
    }
}

/// The important part! Without implementing a [`VoxelRegistry`], you can't use
/// the function.
impl VoxelRegistry for BlockRegistry {
    /// The type of our Voxel, the example uses u16 for Simplicity but you may
    /// have a struct Block { Name: ..., etc ...}, and you'll define that as
    /// the type, but encoding the block data onto simple type like u16 or
    /// u64 is probably preferable.
    type Voxel = Voxel;

    /// The get_mesh function, probably the most important function in the
    /// [`VoxelRegistry`], it is what allows us to  quickly access the Mesh of
    /// each Voxel.
    fn get_mesh(&self, voxel: &Self::Voxel) -> VoxelMesh<&Mesh> {
        match self.get(voxel) {
            Some(mesh) => VoxelMesh::NormalCube(mesh),
            None => VoxelMesh::Null,
        }
    }

    /// Important function that tells our Algorithm if the Voxel is "full", for
    /// example, the Air in minecraft is not "full", but it is still on the
    /// chunk data, to signal there is nothing.
    fn is_covering(&self, voxel: &Self::Voxel, _side: Face) -> bool {
        match voxel {
            Voxel::Air => false,
            _ => true,
        }
    }

    /// The center of the Mesh, out mesh is defined in src/voxel_mesh.rs, just a
    /// constant.
    fn get_center(&self) -> [f32; 3] {
        return [0.0, 0.0, 0.0];
    }

    /// The dimensions of the Mesh, out mesh is defined in src/voxel_mesh.rs,
    /// just a constant.
    fn get_voxel_dimensions(&self) -> [f32; 3] {
        return [1.0, 1.0, 1.0];
    }

    /// The attributes we want to take from out voxels, note that using a lot of
    /// different attributes will likely lead to performance problems and
    /// unpredictable behaviour. We chose these 3 because they are very
    /// common, the algorithm does preserve UV data.
    fn all_attributes(&self) -> Vec<bevy::render::mesh::MeshVertexAttribute> {
        return vec![Mesh::ATTRIBUTE_POSITION, Mesh::ATTRIBUTE_UV_0, Mesh::ATTRIBUTE_NORMAL, Mesh::ATTRIBUTE_COLOR];
    }
}

/// Function to toggle wireframe (seeing the vertices and indices of the mesh).
pub fn toggle_wireframe(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    with: Query<Entity, With<Wireframe>>,
    without: Query<Entity, (Without<Wireframe>, With<MeshemData>)>,
    mut events: EventReader<ToggleWireframe>,
) {
    for _ in events.read() {
        if let Ok(ent) = with.get_single() {
            commands.entity(ent).remove::<Wireframe>();
            for (_, material) in materials.iter_mut() {
                material.base_color.set_alpha(1.0);
            }
        } else if let Ok(ent) = without.get_single() {
            commands.entity(ent).insert(Wireframe);
            for (_, material) in materials.iter_mut() {
                material.base_color.set_alpha(0.0);
            }
        }
    }
}

pub fn meshem_update(
    mut meshem: Query<(&mut VoxelGrid, &Mesh3d, &mut MeshemData)>,
    block_registry: Res<BlockRegistry>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (mut grid, mesh, mut meshem) in &mut meshem {
        let mesh = meshes.get_mut(mesh.id()).unwrap();
        for changed in grid.changed() {
            let linear_index = grid.linearize(*changed);

            let neighbors: [Option<Voxel>; 6] = {
                let mut r = [None; 6];
                for i in 0..6 {
                    match get_neighbor(linear_index as usize, Face::from(i), meshem.data.dims) {
                        None => {},
                        Some(j) => r[i] = Some(grid.linear_voxel(j as i32)),
                    }
                }
                r
            };

            let voxel = grid.voxel(*changed);
            let change =
                if let Voxel::Air = voxel { VoxelChange::Broken } else { VoxelChange::Added };
            meshem.data.log(change, linear_index as usize, voxel, neighbors);
        }

        update_mesh::<Voxel>(mesh, &mut meshem.data, &*block_registry);
        grid.clear_changed();
    }
}

/*
/// System to add or break random voxels.
fn mesh_update(
    mut meshy: Query<(&mut VoxelGrid, &mut MeshemData)>,
    breg: Res<BlockRegistry>,
    mut meshes: ResMut<Assets<Mesh>>,
    mesh_query: Query<&Mesh3d>,
    mut event_reader: EventReader<RegenerateMesh>,
) {
    for _ in event_reader.read() {
        let mesh = meshes
            .get_mut(mesh_query.get_single().unwrap())
            .expect("Couldn't get a mut ref to the mesh");

        let m = meshy.get_single_mut().unwrap().into_inner();
        let mut rng = rand::thread_rng();
        let choice = m.grid.iter().enumerate().choose(&mut rng).unwrap();
        let neighbors: [Option<u16>; 6] = {
            let mut r = [None; 6];
            for i in 0..6 {
                match get_neighbor(choice.0, Face::from(i), m.meta.dims) {
                    None => {},
                    Some(j) => r[i] = Some(m.grid[j]),
                }
            }
            r
        };
        match choice {
            (i, 1) => {
                m.meta.log(VoxelChange::Broken, i, 1, neighbors);
                update_mesh::<Voxel>(mesh, &mut m.meta, breg.into_inner());
                m.grid[i] = 0;
            },
            (i, 0) => {
                m.meta.log(VoxelChange::Added, i, 1, neighbors);
                update_mesh::<Voxel>(mesh, &mut m.meta, breg.into_inner());
                m.grid[i] = 1;
            },
            _ => {},
        }
        break;
    }
}
*/
