//! An example that showcases how to update the mesh.
#[allow(unused_imports, dead_code)]
use bevy::pbr::wireframe::{Wireframe, WireframeConfig, WireframePlugin};
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_meshem::prelude::*;

use super::voxel_grid::{Voxel, VoxelGrid};

pub fn box_mesh(index: u32) -> Mesh {
    generate_voxel_mesh(
        [1.0, 1.0, 1.0],
        [6, 4],
        [
            (Top, [0, index]),
            (Bottom, [0, index]),
            (Right, [2, index]),
            (Left, [3, index]),
            (Back, [4, index]),
            (Forward, [5, index]),
        ],
        [0.0, 0.0, 0.0],
        0.01,
        Some(1.0),
        1.0,
    )
}

impl Voxel {
    pub fn box_mesh(&self) -> Option<Mesh> {
        match self {
            Voxel::Air => None,
            Voxel::Grass => Some(box_mesh(0)),
            Voxel::Stone => Some(box_mesh(1)),
            Voxel::Dirt => Some(box_mesh(2)),
            Voxel::Base => Some(box_mesh(3)),
            _ => None,
        }
    }
}

pub struct BoxMeshPlugin;
impl Plugin for BoxMeshPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BlockRegistry::new());
            //.insert_resource(AmbientLight { brightness: 400.0, color: Color::WHITE });

        app.add_systems(Update, (setup_meshem, meshem_update));
    }
}

#[derive(Component)]
pub struct MeshemData {
    data: MeshMD<Voxel>,
}

#[derive(Component)]
pub struct Meshem;

#[derive(Resource)]
pub struct BlockRegistry {
    meshes: HashMap<Voxel, Mesh>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        let mut registry = Self { meshes: HashMap::new() };
        for voxel in Voxel::iter() {
            registry.add(voxel);
        }
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
        let texture_mesh = asset_server.load("texture_map.png");

        let (culled_mesh, metadata) =
            mesh_grid::<Voxel>(dims, &[], &grid.voxels, &*breg, MeshingAlgorithm::Culling, Some(SmoothLightingParameters {
                smoothing: 1.0,
                apply_at_gen: false,
                intensity: 0.5,
                max: 0.6,
            }))
                .unwrap();
        let culled_mesh_handle: Handle<Mesh> = meshes.add(culled_mesh.clone());

        commands.entity(grid_entity)
            .insert((
                Transform {
                    scale: Vec3::new(1.0, 0.1, 1.0),
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
                    base_color: Color::WHITE,
                    base_color_texture: Some(texture_mesh),
                    ..default()
                })),
            ));
    }
}


pub fn meshem_update(
    mut meshem: Query<(&mut VoxelGrid, &mut MeshemData, &Children), Changed<VoxelGrid>>,
    mesh3ds: Query<&Mesh3d>,
    block_registry: Res<BlockRegistry>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (mut grid, mut meshem, children) in &mut meshem {
        let mut mesh: Option<&Mesh3d> = None;
        for child in children {
            if let Ok(child_mesh) = mesh3ds.get(*child) {
                mesh = Some(child_mesh);
            }
        }

        let Some(mesh) = mesh else { continue; };
        let mesh = meshes.get_mut(mesh.id()).unwrap();
        for change in grid.changed() {
            let linear_index = grid.linearize(change.point);

            let neighbors: [Option<Voxel>; 6] = {
                let mut r = [None; 6];
                for i in 0..6 {
                    match get_neighbor(linear_index as usize, Face::from(i), meshem.data.dims) {
                        None => {},
                        Some(j) => match grid.linear_voxel(j as i32) {
                            Voxel::Air => {},
                            voxel => r[i] = Some(voxel),
                        }
                    }
                }
                r
            };

            let (voxel, meshem_change) =
                if let Voxel::Air = change.new_voxel { 
                    (change.last_voxel, VoxelChange::Broken)
                } else {
                    (change.new_voxel, VoxelChange::Added)
                };
            meshem.data.log(meshem_change, linear_index as usize, voxel, neighbors);
        }

        update_mesh::<Voxel>(mesh, &mut meshem.data, &*block_registry);

        let dims: Dimensions = {
            let array = grid.array();
            (array[0] as usize, array[1] as usize, array[2] as usize)
        };
        apply_smooth_lighting(&*block_registry, mesh, &meshem.data, dims, 0, grid.size() as usize, &grid.voxels);
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
