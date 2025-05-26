use std::ffi::OsStr;
use std::sync::Arc;

use bevy::asset::LoadedFolder;
use bevy::pbr::{
    ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, ShaderRef, TextureDimension, TextureFormat,
};
use bevy::render::storage::ShaderStorageBuffer;
use bgm::Face;
use binary_greedy_meshing as bgm;
use dashmap::DashMap;

use super::mesh_chunks::ATTRIBUTE_VOXEL_DATA;
// use super::{BlockTexState, BlockTextureFolder};
use crate::voxel::Voxel;
// use crate::render::parse_block_tex_name;

pub struct TextureArrayPlugin;

fn load_block_textures(mut commands: Commands, asset_server: Res<AssetServer>) {
    // load multiple, individual sprites from a folder
    commands.insert_resource(BlockTextureFolder(asset_server.load_folder("textures/blocks")));
}

fn check_block_textures(
    mut next_state: ResMut<NextState<BlockTexState>>,
    texture_folder: ResMut<BlockTextureFolder>,
    mut events: EventReader<AssetEvent<LoadedFolder>>,
) {
    // Advance the `AppState` once all sprite handles have been loaded by the
    // `AssetServer`
    for event in events.read() {
        if event.is_loaded_with_dependencies(&texture_folder.0) {
            next_state.set(BlockTexState::Loaded);
        }
    }
}

const DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
pub fn parse_block_tex_name(filename: &OsStr) -> Option<(Voxel, FaceSpecifier)> {
    let filename = filename.to_str()?.trim_end_matches(DIGITS);
    let (block_name, face) = match filename.rsplit_once("_") {
        Some((block, "side")) => (block, FaceSpecifier::Side),
        Some((block, "bottom")) => (block, FaceSpecifier::Specific(Face::Down)),
        Some((block, "top")) => (block, FaceSpecifier::Specific(Face::Up)),
        Some((block, "front")) => (block, FaceSpecifier::Specific(Face::Front)),
        Some((block, "back")) => (block, FaceSpecifier::Specific(Face::Back)),
        _ => (filename, FaceSpecifier::All),
    };
    Some((Voxel::from_name(block_name)?, face))
}

impl Plugin for TextureArrayPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<BlockTexState>();

        app.insert_resource(TextureMap(Arc::new(DashMap::new())))
            .add_plugins(
                MaterialPlugin::<ExtendedMaterial<StandardMaterial, ArrayTextureMaterial>>::default(
                ),
            )
            .add_systems(OnEnter(BlockTexState::Setup), load_block_textures)
            .add_systems(Update, check_block_textures.run_if(in_state(BlockTexState::Setup)))
            .add_systems(OnEnter(BlockTexState::Loaded), build_tex_array);
    }
}

const UP_SPECIFIER: [FaceSpecifier; 2] = [FaceSpecifier::Specific(Face::Up), FaceSpecifier::All];
const DOWN_SPECIFIER: [FaceSpecifier; 3] =
    [FaceSpecifier::Specific(Face::Down), FaceSpecifier::Specific(Face::Up), FaceSpecifier::All];
const LEFT_SPECIFIER: [FaceSpecifier; 3] =
    [FaceSpecifier::Specific(Face::Left), FaceSpecifier::Side, FaceSpecifier::All];
const RIGHT_SPECIFIER: [FaceSpecifier; 3] =
    [FaceSpecifier::Specific(Face::Right), FaceSpecifier::Side, FaceSpecifier::All];
const FRONT_SPECIFIER: [FaceSpecifier; 3] =
    [FaceSpecifier::Specific(Face::Front), FaceSpecifier::Side, FaceSpecifier::All];
const BACK_SPECIFIER: [FaceSpecifier; 3] =
    [FaceSpecifier::Specific(Face::Back), FaceSpecifier::Side, FaceSpecifier::All];

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum FaceSpecifier {
    Specific(Face),
    Side,
    All,
}

pub trait Specifiers {
    fn specifiers(&self) -> &[FaceSpecifier];
}

impl Specifiers for Face {
    fn specifiers(&self) -> &[FaceSpecifier] {
        match self {
            Face::Up => &UP_SPECIFIER,
            Face::Down => &DOWN_SPECIFIER,
            Face::Left => &LEFT_SPECIFIER,
            Face::Right => &RIGHT_SPECIFIER,
            Face::Front => &FRONT_SPECIFIER,
            Face::Back => &BACK_SPECIFIER,
        }
    }
}

#[derive(Resource)]
pub struct TextureMap(pub Arc<DashMap<(Voxel, FaceSpecifier), usize>>);

pub trait TextureMapTrait {
    fn get_texture_index(&self, block: Voxel, face: Face) -> usize;
}

impl TextureMapTrait for &DashMap<(Voxel, FaceSpecifier), usize> {
    // TODO: need to allow the user to create a json with "texture files links" such
    // as: grass_block_bottom.png -> dirt.png
    // furnace_bottom.png -> stone.png
    // etc ...
    fn get_texture_index(&self, block: Voxel, face: Face) -> usize {
        for specifier in face.specifiers() {
            if let Some(i) = self.get(&(block, *specifier)) {
                return *i;
            }
        }
        0
    }
}

fn missing_tex(model: &Image) -> Image {
    let mut img = Image::new_fill(
        Extent3d { width: model.width(), height: model.width(), ..Default::default() },
        TextureDimension::D2,
        &[130, 130, 130, 255],
        model.texture_descriptor.format,
        RenderAssetUsages::default(),
    );
    let w = model.width();
    let pixels = w * w;
    let half_w = w / 2;
    for i in 0..pixels {
        let (x, y) = ((i % w) / half_w, i / (w * half_w));
        if x != y {
            continue;
        }
        img.set_color_at(x, y, Color::srgb(1., 0.5, 0.5));
    }
    img
}

fn build_tex_array(
    mut commands: Commands,
    block_textures: Res<BlockTextureFolder>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    mut textures: ResMut<Assets<Image>>,
    texture_map: Res<TextureMap>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, ArrayTextureMaterial>>>,
    mut next_state: ResMut<NextState<BlockTexState>>,
    mut shader_buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    let mut texture_list: Vec<&Image> = Vec::new();
    let mut anim_offsets = vec![1];
    let mut index = 1;
    let loaded_folder: &LoadedFolder = loaded_folders.get(&block_textures.0).unwrap();
    for handle in loaded_folder.handles.iter() {
        let id = handle.id().typed_unchecked::<Image>();
        let Some(texture) = textures.get(id) else {
            warn!("{:?} did not resolve to an `Image` asset.", handle.path().unwrap());
            continue;
        };
        let filename = handle.path().unwrap().path().file_stem().unwrap();
        let Some((block, face_specifier)) = parse_block_tex_name(filename) else {
            continue;
        };
        let frames = texture.height() / texture.width();
        texture_map.0.insert((block, face_specifier), index);
        texture_list.push(texture);
        for _ in 0..frames {
            anim_offsets.push(frames);
            index += 1;
        }
    }
    let default = Image::new_fill(
        Extent3d { width: 2, height: 2, ..Default::default() },
        TextureDimension::D2,
        &[100, 100, 25, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    let model = texture_list.get(0).cloned().unwrap_or(&default);
    let missing_tex = missing_tex(model);
    texture_list.insert(0, &missing_tex);
    let array_tex = Image::new(
        Extent3d {
            width: model.width(),
            height: model.height(),
            depth_or_array_layers: index as u32,
        },
        TextureDimension::D2,
        texture_list.into_iter().flat_map(|tex| tex.data.clone().unwrap()).collect(),
        model.texture_descriptor.format,
        RenderAssetUsages::default(),
    );
    let handle = textures.add(array_tex);
    let handle = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            perceptual_roughness: 1.,
            reflectance: 0.1,
            alpha_mode: AlphaMode::AlphaToCoverage,
            ..Default::default()
        },
        extension: ArrayTextureMaterial {
            array_texture: handle,
            anim_offsets: shader_buffers.add(ShaderStorageBuffer::from(anim_offsets)),
        },
    });
    commands.insert_resource(BlockTextureArray(handle));
    next_state.set(BlockTexState::Mapped);
}

#[derive(Resource)]
pub struct BlockTextureArray(pub Handle<ExtendedMaterial<StandardMaterial, ArrayTextureMaterial>>);

#[derive(Asset, AsBindGroup, Debug, Clone, TypePath)]
pub struct ArrayTextureMaterial {
    #[texture(100, dimension = "2d_array")]
    #[sampler(101)]
    array_texture: Handle<Image>,
    #[storage(102, read_only)]
    anim_offsets: Handle<ShaderStorageBuffer>,
}

impl MaterialExtension for ArrayTextureMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/chunk.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/chunk.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<ArrayTextureMaterial>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[ATTRIBUTE_VOXEL_DATA.at_shader_location(0)])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}
