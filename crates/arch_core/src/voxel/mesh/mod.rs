use bevy::{platform::collections::HashSet, prelude::*};

pub use chunk::{Scalar, VoxelChunk, padded, unpadded};

use crate::voxel::{Voxel, VoxelAabb, Voxels};

// Data
pub mod chunk;

// Meshing
pub mod binary_greedy;
pub mod surface_net;

// Perf control
pub mod remesh;

pub use binary_greedy::BinaryGreedy;
pub use surface_net::SurfaceNet;
pub use remesh::Remesh;

#[derive(SystemSet, Copy, Clone, Debug, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct UpdateVoxelMeshSet;

pub fn plugin(app: &mut App) {
    app.add_event::<ChangedChunks>();

    app.add_plugins(surface_net::SurfaceNetPlugin);
    app.add_plugins(remesh::plugin);
    // app.add_plugins(ass_mesh::ASSMeshPlugin);
    // app.add_plugins(meshem::MeshemPlugin);
    app.add_plugins(binary_greedy::plugin);

    app.add_systems(PostUpdate, clear_changed_chunks.before(UpdateVoxelMeshSet));
}

#[derive(Event, Debug)]
pub struct ChangedChunks {
    pub voxel_entity: Entity,
    pub changed_chunks: Vec<IVec3>,
}

pub fn clear_changed_chunks(
    mut voxels: Query<(Entity, &mut Voxels)>,
    mut writer: EventWriter<ChangedChunks>,
) {
    for (voxel_entity, mut voxels) in &mut voxels {
        writer.write(ChangedChunks {
            voxel_entity,
            changed_chunks: voxels.render_chunks.changed_chunk_pos_iter().collect::<Vec<_>>(),
        });
        voxels.render_chunks.clear_changed_chunks();
    }
}

const CHUNK_SIZE: IVec3 = IVec3::splat(unpadded::SIZE as Scalar);
const CHUNK_SIZE_FLOAT: Vec3 = Vec3::splat(unpadded::SIZE as f32);

#[derive(PartialEq, Eq, Clone)]
pub struct RenderChunks {
    // chunks: HashMap<IVec3, VoxelChunk>, // spatially hashed chunks because its easy
    pub chunks: Vec<VoxelChunk>, // linearize chunks similar to the voxels in the chunk
    pub strides: [usize; 3],
    pub changed_chunks: HashSet<IVec3>,

    pub chunk_size: IVec3,
}

impl RenderChunks {
    pub fn new(voxel_size: IVec3) -> Self {
        let chunk_size = (voxel_size / IVec3::splat(unpadded::SIZE as Scalar)) + IVec3::ONE;
        // let mut chunks = HashMap::with_capacity((size.x * size.y * size.z) as usize);
        // for z in 0..size.z {
        //     for x in 0..size.x {
        //         for y in 0..size.y {
        //             chunks.insert(IVec3::new(x, y, z), VoxelChunk::new());
        //         }
        //     }
        // }
        Self {
            chunks: vec![VoxelChunk::new(); (chunk_size.x * chunk_size.y * chunk_size.z) as usize],
            strides: [1, chunk_size.z as usize, (chunk_size.z * chunk_size.x) as usize],
            changed_chunks: default(),
            // update_voxels: default(),
            chunk_size,
        }
    }

    #[inline]
    pub fn get_voxel(&self, point: IVec3) -> Voxel {
        #[cfg(feature = "trace")]
        let get_voxel_span = info_span!("get_render_voxel");

        let chunk_point = Self::find_chunk(point);
        if let Some(chunk) = self.get_chunk(chunk_point) {
            chunk.get_voxel(Self::relative_point(chunk_point, point)).unwrap_or(Voxel::Barrier)
        } else {
            Voxel::Barrier
        }
    }

    #[inline]
    pub fn set_voxel(&mut self, point: IVec3, voxel: Voxel) {
        #[cfg(feature = "trace")]
        let set_voxel_span = info_span!("set_render_voxel");

        // if !self.clip.contains_point(point) {
        //     warn!("attempted voxel set at clip boundary");
        //     return;
        // }

        // if point.x < 0
        //     || point.y < 0
        //     || point.z < 0
        //     || point.x >= self.size.x * unpadded::SIZE_SCALAR
        //     || point.y >= self.size.y * unpadded::SIZE_SCALAR
        //     || point.z >= self.size.z * unpadded::SIZE_SCALAR
        // {
        //     return;
        // }

        // if !Voxels::is_boundary_point(point) {
        //     #[cfg(feature = "trace")]
        //     let set_voxel_nonboundary_span = info_span!("set_voxel_nonboundary");

        //     let chunk_point = Self::find_chunk(point);
        //     let chunk = self.chunks.entry(chunk_point).or_default();
        //     let relative_point = Self::relative_point_unoriented(point);
        //     chunk.set(relative_point.into(), voxel);
        //     self.changed_chunks.insert(chunk_point); // negligible
        // } else {
        // Set the overlapping chunks boundary voxels as well
        // setting overlap chunks adds about 10% to the simulation time
        // }

        self.set_voxel_chunk_overlap(point, voxel);
    }

    #[inline]
    pub fn get_chunk(&self, chunk_point: IVec3) -> Option<&VoxelChunk> {
        let chunk_index = self.chunk_index(chunk_point)?;
        self.chunks.get(chunk_index)
        // self.chunks.get(&chunk_point)
    }

    pub fn get_chunk_mut(&mut self, chunk_point: IVec3) -> Option<&mut VoxelChunk> {
        let chunk_index = self.chunk_index(chunk_point)?;
        self.chunks.get_mut(chunk_index)
        // self.chunks.get_mut(&chunk_point)
    }

    // pub fn relative_point(point: IVec3) -> IVec3 {
    //     point.rem_euclid(IVec3::splat(unpadded::SIZE as Scalar))
    // }

    #[inline]
    pub fn relative_point_unoriented(point: IVec3) -> IVec3 {
        point % CHUNK_SIZE
    }

    #[inline]
    pub fn is_boundary_point(point: IVec3) -> bool {
        let relative_point = Self::relative_point_unoriented(point);
        relative_point.x == 0
            || relative_point.x == unpadded::SIZE_SCALAR
            || relative_point.y == 0
            || relative_point.y == unpadded::SIZE_SCALAR
            || relative_point.z == 0
            || relative_point.z == unpadded::SIZE_SCALAR
    }

    pub fn get_relative_points(
        points: impl Iterator<Item = IVec3>,
    ) -> impl Iterator<Item = (IVec3, IVec3)> {
        points.flat_map(|point| {
            Self::chunks_overlapping_voxel(point).map(move |chunk_point| {
                (chunk_point, Self::relative_point_with_boundary(chunk_point, point))
            })
        })
    }

    #[inline]
    pub fn set_voxel_chunk_overlap(&mut self, point: IVec3, voxel: Voxel) {
        #[cfg(feature = "trace")]
        let set_voxel_overlap_span = info_span!("set_voxel_overlap_loop");

        for chunk_point in Self::chunks_overlapping_voxel(point) {
            #[cfg(feature = "trace")]
            let set_voxel_single_chunk_span = info_span!("set_voxel_single_chunk");

            let Some(chunk) = self.get_chunk_mut(chunk_point) else {
                continue;
            };
            let relative_point = Self::relative_point_with_boundary(chunk_point, point);
            if chunk.in_chunk_bounds_unpadded(relative_point) {
                chunk.set_unpadded(relative_point.into(), voxel);
                self.changed_chunks.insert(chunk_point); // negligible
            }
        }
    }

    /// Given a voxel position, find the chunk it is in.
    #[inline]
    pub fn find_chunk(point: IVec3) -> IVec3 {
        #[cfg(feature = "trace")]
        let find_chunk_span = info_span!("find_chunk");

        // point.div_euclid(CHUNK_SIZE)
        point / CHUNK_SIZE
    }

    #[inline]
    pub fn chunk_index(&self, chunk_point: IVec3) -> Option<usize> {
        if chunk_point.x < 0
            || chunk_point.y < 0
            || chunk_point.z < 0
            || chunk_point.x >= self.chunk_size.x
            || chunk_point.y >= self.chunk_size.y
            || chunk_point.z >= self.chunk_size.z
        {
            return None;
        }

        Some(
            chunk_point.z as usize
                + chunk_point.x as usize * self.strides[1]
                + chunk_point.y as usize * self.strides[2],
        )
    }

    // pub fn chunk_delinearize(&self, chunk_index: usize) -> IVec3 {
    //     let z = chunk_index % self.strides[1];
    //     let x = (chunk_index / self.strides[1]) % self.strides[2];
    //     let y = chunk_index / self.strides[2];
    //     IVec3::new(x, y, z)
    // }

    #[inline]
    pub fn chunks_overlapping_voxel(voxel_pos: IVec3) -> impl Iterator<Item = IVec3> {
        #[cfg(feature = "trace")]
        let chunks_overlapping_voxel_span = info_span!("chunks_overlapping_voxel");

        let min_chunk =
            ((voxel_pos - IVec3::splat(2)).as_vec3() / CHUNK_SIZE_FLOAT).floor().as_ivec3();
        let max_chunk =
            ((voxel_pos + IVec3::splat(2)).as_vec3() / CHUNK_SIZE_FLOAT).ceil().as_ivec3();

        (min_chunk.y..max_chunk.y).flat_map(move |y| {
            (min_chunk.x..max_chunk.x)
                .flat_map(move |x| (min_chunk.z..max_chunk.z).map(move |z| IVec3::new(x, y, z)))
        })
    }

    #[inline]
    pub fn relative_point(chunk: IVec3, world_point: IVec3) -> IVec3 {
        let chunk_origin = chunk * unpadded::SIZE as Scalar;
        world_point - chunk_origin
    }

    #[inline]
    pub fn relative_point_with_boundary(chunk: IVec3, world_point: IVec3) -> IVec3 {
        Self::relative_point(chunk, world_point) + IVec3::ONE
    }

    // [min, max]
    pub fn chunk_bounds(&self) -> (IVec3, IVec3) {
        // let mut min = IVec3::MAX;
        // let mut max = IVec3::MIN;

        // if self.chunks.len() == 0 {
        //     return (IVec3::ZERO, IVec3::ZERO);
        // }

        // for chunk_point in self.chunks.keys().copied() {
        //     min = min.min(chunk_point);
        //     max = max.max(chunk_point + IVec3::splat(1));
        // }

        // (min, max)
        (IVec3::ZERO, self.chunk_size)
    }

    pub fn chunk_aabb(&self) -> VoxelAabb {
        let (min, max) = self.chunk_bounds();
        VoxelAabb::new(min, max)
    }

    pub fn chunk_size(&self) -> IVec3 {
        // let (min, max) = self.chunk_bounds();
        // max - min
        self.chunk_size
    }

    pub fn chunk_pos_iter<'a, 'b>(&'a self) -> impl Iterator<Item = IVec3> + use<'b> {
        // self.chunks.keys().copied()
        let chunk_size = self.chunk_size;
        (0..chunk_size.z).flat_map(move |z| {
            (0..chunk_size.x)
                .flat_map(move |x| (0..chunk_size.y).map(move |y| IVec3::new(x, y, z)))
        })
    }

    pub fn point_iter(&self) -> impl Iterator<Item = IVec3> {
        self.chunk_pos_iter().flat_map(move |chunk_point| {
            let chunk_base = chunk_point * unpadded::SIZE as Scalar;
            VoxelChunk::point_iter().map(move |point| chunk_base + IVec3::from(point))
        })
    }

    pub fn chunk_iter(&self) -> impl Iterator<Item = (IVec3, &VoxelChunk)> {
        self.chunk_pos_iter().map(|chunk_point| (chunk_point, self.get_chunk(chunk_point).unwrap()))

    }
    // pub fn chunk_iter_mut<'a, 'b: 'a>(&'b mut self) -> impl Iterator<Item = (IVec3, &'a mut VoxelChunk)> + use<'b> {
    pub fn chunk_iter_mut(&mut self) -> impl Iterator<Item = &mut VoxelChunk>{
        self.chunks.iter_mut()
        // self.chunk_pos_iter().map(move |chunk_point| (chunk_point, self.get_chunk_mut(chunk_point).unwrap()))
    }

    pub fn changed_chunk_pos_iter(&self) -> impl Iterator<Item = IVec3> {
        self.changed_chunks.iter().copied()
    }

    pub fn changed_chunk_iter(&self) -> impl Iterator<Item = (IVec3, &VoxelChunk)> {
        self.changed_chunks.iter().filter_map(|&p| self.get_chunk(p).map(|chunk| (p, chunk)))
    }

    pub fn clear_changed_chunks(&mut self) {
        self.changed_chunks.clear();
    }
}

impl std::fmt::Debug for RenderChunks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RenderChunks")
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn find_chunk() {
        assert_eq!(RenderChunks::find_chunk(ivec3(0, 0, 0)), ivec3(0, 0, 0));
        assert_eq!(RenderChunks::find_chunk(ivec3(63, 0, 0)), ivec3(1, 0, 0));
        // assert_eq!(RenderChunks::find_chunk(ivec3(-1, 0, 0)), ivec3(-1, 0, 0));
        // assert_eq!(RenderChunks::find_chunk(ivec3(-62, 0, 0)), ivec3(-1, 0, 0));
        // assert_eq!(RenderChunks::find_chunk(ivec3(-63, 0, 0)), ivec3(-2, 0, 0));
    }

    #[test]
    fn find_chunk_relative() {
        assert_eq!(RenderChunks::relative_point(ivec3(0, 0, 0), ivec3(0, 0, 0)), ivec3(0, 0, 0));
        assert_eq!(RenderChunks::relative_point(ivec3(0, 0, 0), ivec3(61, 0, 0)), ivec3(61, 0, 0));
        assert_eq!(RenderChunks::relative_point(ivec3(0, 0, 0), ivec3(62, 0, 0)), ivec3(62, 0, 0)); // oob
        assert_eq!(RenderChunks::relative_point(ivec3(0, 0, 0), ivec3(63, 0, 0)), ivec3(63, 0, 0)); // oob
        assert_eq!(RenderChunks::relative_point(ivec3(1, 0, 0), ivec3(62, 0, 0)), ivec3(0, 0, 0));
        assert_eq!(RenderChunks::relative_point(ivec3(1, 0, 0), ivec3(63, 0, 0)), ivec3(1, 0, 0));

        // // negative handling
        // assert_eq!(
        //     RenderChunks::relative_point(ivec3(0, 0, 0), ivec3(-1, -1, -1)),
        //     ivec3(-1, -1, -1)
        // );
        // assert_eq!(
        //     RenderChunks::relative_point(ivec3(-1, -1, -1), ivec3(0, 0, 0)),
        //     ivec3(62, 62, 62)
        // ); // oob
        // assert_eq!(
        //     RenderChunks::relative_point(ivec3(-1, -1, -1), ivec3(-1, -1, -1)),
        //     ivec3(61, 61, 61)
        // );
        // assert_eq!(
        //     RenderChunks::relative_point(ivec3(-1, -1, -1), ivec3(-62, -62, -62)),
        //     ivec3(0, 0, 0)
        // );
    }

    #[test]
    fn find_chunk_relative_unpadded() {
        assert_eq!(
            RenderChunks::relative_point_with_boundary(ivec3(0, 0, 0), ivec3(0, 0, 0)),
            ivec3(1, 1, 1)
        );
        assert_eq!(
            RenderChunks::relative_point_with_boundary(ivec3(0, 0, 0), ivec3(62, 0, 0)),
            ivec3(63, 1, 1)
        ); // oob
        assert_eq!(
            RenderChunks::relative_point_with_boundary(ivec3(0, 0, 0), ivec3(63, 0, 0)),
            ivec3(64, 1, 1)
        ); // oob
        assert_eq!(
            RenderChunks::relative_point_with_boundary(ivec3(1, 0, 0), ivec3(61, 0, 0)),
            ivec3(0, 1, 1)
        );
        assert_eq!(
            RenderChunks::relative_point_with_boundary(ivec3(1, 0, 0), ivec3(62, 0, 0)),
            ivec3(1, 1, 1)
        );

        assert_eq!(
            RenderChunks::relative_point_with_boundary(ivec3(0, 0, 0), ivec3(61, 61, 61)),
            ivec3(62, 62, 62)
        );
        assert_eq!(
            RenderChunks::relative_point_with_boundary(ivec3(1, 1, 1), ivec3(61, 61, 61)),
            ivec3(0, 0, 0)
        );

        // negative handling
        assert_eq!(
            RenderChunks::relative_point_with_boundary(ivec3(0, 0, 0), ivec3(-1, -1, -1)),
            ivec3(0, 0, 0)
        );
        assert_eq!(
            RenderChunks::relative_point_with_boundary(ivec3(-1, -1, -1), ivec3(0, 0, 0)),
            ivec3(63, 63, 63)
        );
        assert_eq!(
            RenderChunks::relative_point_with_boundary(ivec3(-1, -1, -1), ivec3(-1, -1, -1)),
            ivec3(62, 62, 62)
        );
        assert_eq!(
            RenderChunks::relative_point_with_boundary(ivec3(-1, -1, -1), ivec3(-62, -62, -62)),
            ivec3(1, 1, 1)
        );
        assert_eq!(
            RenderChunks::relative_point_with_boundary(ivec3(-1, -1, -1), ivec3(-63, -63, -63)),
            ivec3(0, 0, 0)
        );
    }

    // #[test]
    // fn set_voxel_batch() {
    //     // just make sure the batch actually does the same thing as setting directly
    //     let size = 1;
    //     let len = size * size * size;
    //     let point_iter = (-size..=size).flat_map(move |y| {
    //         (-size..=size).flat_map(move |x| (-size..=size).map(move |z| IVec3::new(x, y, z)))
    //     });
    //     let voxel_iter = (-len..len).map(|_| Voxel::Sand);

    //     let mut voxels_direct = Voxels::new(IVec3::splat(size));
    //     for (point, voxel) in point_iter.clone().zip(voxel_iter.clone()) {
    //         voxels_direct.set_voxel(point, voxel);
    //     }

    //     let mut voxels_batch = Voxels::new(IVec3::splat(size));
    //     voxels_batch.set_voxels(point_iter.clone(), voxel_iter.clone());

    //     let diff = voxels_direct.diff(&voxels_batch, 50);
    //     if diff.len() > 0 {
    //         panic!("diffs: {:?}", diff);
    //     }
    // }
}
