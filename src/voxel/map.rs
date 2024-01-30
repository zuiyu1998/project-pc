use bevy::{
    prelude::*,
    render::mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues},
    utils::HashMap,
};

use bevy_xpbd_3d::components::Collider;
use fast_surface_nets::{
    ndshape::{ConstShape, ConstShape3u32},
    SignedDistance,
};
use fast_surface_nets::{surface_nets, SurfaceNetsBuffer};

use noise::{Fbm, NoiseFn, Perlin};
use std::{cmp::Eq, mem, ops::Div};
use std::{
    sync::mpsc::{channel, Receiver, Sender},
    thread::JoinHandle,
};

use super::{SdfValue, TerrainMaterial, VoxelMaterial};

type ChunkShape = ConstShape3u32<18, 18, 18>;

//单独线程做数据的修改，修改完成后将数据发送给bevy的实体
pub enum VoxelEvent {
    Drop,
    ChunkSpawn(ChunkPosition, Entity),
    None,
}

pub type VoxelEventSender = Sender<VoxelEvent>;
pub type VoxelEventReceiver = Receiver<VoxelEvent>;

pub type MeshSender = Sender<(Option<Mesh>, Option<Collider>, ChunkPosition)>;
pub type MeshReceiver = Receiver<(Option<Mesh>, Option<Collider>, ChunkPosition)>;

#[derive(Debug, Hash, PartialEq, Eq, Reflect, Deref, DerefMut, Component, Clone)]
pub struct ChunkPosition(pub IVec3);

impl ChunkPosition {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        ChunkPosition(IVec3::new(x, y, z))
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct VoxelEventSenderResource(VoxelEventSender);

#[derive(Deref, DerefMut, Default)]
pub struct MeshReceiverResource(pub Option<MeshReceiver>);

#[derive(Resource, Deref, DerefMut)]
pub struct VoxelDataHandle(JoinHandle<()>);

pub fn start_handle_voxel_data_thread() -> (
    VoxelEventSenderResource,
    MeshReceiverResource,
    VoxelDataHandle,
) {
    let (mesh_sender, mesh_reciver) = channel();
    let (event_sender, event_reciver) = channel();

    let mut voxel_data = VoxelData::new(event_reciver, mesh_sender);

    let handle = std::thread::spawn(move || {
        voxel_data.run();
    });

    (
        VoxelEventSenderResource(event_sender),
        MeshReceiverResource(Some(mesh_reciver)),
        VoxelDataHandle(handle),
    )
}

#[derive(Resource, Deref, DerefMut)]

pub struct SpawnMeshs(Vec<ChunkPosition>);

//mesh 缓存，避免短时间大量的实体生成
#[derive(Resource)]
pub struct MeshCache {
    data: Vec<(Mesh, Collider, ChunkPosition)>,
    //最大可输出事件
    max_pop: usize,
    is_busy: isize,
}

impl Default for MeshCache {
    fn default() -> Self {
        Self {
            data: vec![],
            max_pop: 16,
            is_busy: 16,
        }
    }
}

impl MeshCache {
    pub fn set_max_pop(&mut self, max_pop: usize) {
        self.max_pop = max_pop;
        self.is_busy = max_pop as isize;
    }

    pub fn push(&mut self, event: (Mesh, Collider, ChunkPosition)) {
        self.tick(1);

        self.data.push(event);
    }

    pub fn pop(&mut self) -> Option<Vec<(Mesh, Collider, ChunkPosition)>> {
        self.tick(-1);
        if !self.is_busy() {
            let mut empty = vec![];
            mem::swap(&mut self.data, &mut empty);

            return Some(empty);
        } else {
            if self.data.len() > self.max_pop {
                let mut next = self.data.split_off(self.max_pop);
                mem::swap(&mut self.data, &mut next);

                return Some(next);
            } else {
                return None;
            }
        }
    }

    pub fn tick(&mut self, work: isize) {
        self.is_busy += work;
        self.is_busy = self.is_busy.clamp(0, self.max_pop as isize);
    }

    pub fn is_busy(&self) -> bool {
        self.is_busy > 0
    }
}

impl Default for SpawnMeshs {
    fn default() -> Self {
        let mut positions = vec![];
        let n = 8;

        for x in -n..n {
            for y in -n..n {
                for z in -n..n {
                    positions.push(ChunkPosition(IVec3::new(x, y, z)));
                }
            }
        }

        SpawnMeshs(positions)
    }
}

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
pub struct Map {
    pub loading_meshs: HashMap<ChunkPosition, Entity>,
    pub meshs: HashMap<ChunkPosition, Entity>,
    pub voxel_terrain_material: Handle<TerrainMaterial>,
}

pub struct ChunkData {
    pub sdf: [SdfValue; ChunkShape::USIZE],
    pub position: ChunkPosition,
}

impl ChunkData {
    pub fn new(position: ChunkPosition) -> Self {
        let seed = 100;
        // let perlin = Perlin::new(seed);
        let mut fbm = Fbm::<Perlin>::new(seed);
        // fbm.frequency = 0.2;
        // fbm.lacunarity = 0.2;
        fbm.octaves = 4;

        let mut sdf = [SdfValue::default(); ChunkShape::USIZE];
        for i in 0u32..ChunkShape::SIZE {
            let [x, y, z] = ChunkShape::delinearize(i);

            let p = IVec3::new(x as i32, y as i32, z as i32) + position.0 * VoxelData::MESH as i32;

            let f_terr = fbm.get(p.xz().as_dvec2().div(129.).to_array()) as f32;
            let f_3d = fbm.get(p.as_dvec3().div(70.).to_array()) as f32;

            let mut value = (p.y as f32) / 12. - f_terr - f_3d * 2.5;

            let mut material_id = VoxelMaterial::STONE;
            if p.y < 0 && value > 0. {
                value = -0.1;
                material_id = VoxelMaterial::WATER;
            }

            sdf[i as usize] = SdfValue::new(value, material_id);
        }

        let perlin = Perlin::new(123);

        for lx in 0..VoxelData::MESH as u32 {
            for lz in 0..VoxelData::MESH as u32 {
                let mut air_dist = 0;

                for ly in (0..VoxelData::MESH as u32).rev() {
                    //chunk 相邻数据可能会不一样？
                    let index = ChunkShape::linearize([lx, ly, lz]);

                    let p = IVec3::new(lx as i32, ly as i32, lz as i32);

                    let mut c = sdf[index as usize];

                    if c.is_negative() {
                        air_dist = 0;
                    } else {
                        air_dist += 1;
                    }

                    if c.material_id == VoxelMaterial::STONE {
                        let mut replace = c.material_id;
                        if p.y < 2
                            && air_dist <= 2
                            && perlin.get([p.x as f64 / 32., p.z as f64 / 32.]) > 0.1
                        {
                            replace = VoxelMaterial::SAND;
                        } else if air_dist <= 1 {
                            replace = VoxelMaterial::GRASS;
                        } else if air_dist < 3 {
                            replace = VoxelMaterial::DIRT;
                        }
                        c.material_id = replace;
                    }

                    sdf[index as usize] = c;
                }
            }
        }

        Self { sdf, position }
    }
}

pub struct VoxelData {
    chunk_data: HashMap<ChunkPosition, ChunkData>,
    mesh_sender: MeshSender,
    event_reciver: VoxelEventReceiver,
    surface_nets_buffer: SurfaceNetsBuffer,
}

impl VoxelData {
    pub fn new(event_reciver: VoxelEventReceiver, mesh_sender: MeshSender) -> Self {
        VoxelData {
            chunk_data: Default::default(),
            mesh_sender,
            event_reciver,
            surface_nets_buffer: SurfaceNetsBuffer::default(),
        }
    }
}

impl VoxelData {
    pub const MESH: usize = 16;

    pub fn handle_chunk_spawn(&mut self, position: ChunkPosition, entity: Entity) {
        let start = std::time::Instant::now();
        info!("{:?}, {:?}", position, entity);

        if !self.chunk_data.contains_key(&position) {
            let chunk_data = ChunkData::new(position.to_owned());
            self.chunk_data.insert(position.to_owned(), chunk_data);
        }

        let chunk_data = self.chunk_data.get(&position).unwrap();

        surface_nets(
            &chunk_data.sdf,
            &ChunkShape {},
            [0; 3],
            [17; 3],
            &mut self.surface_nets_buffer,
        );

        if self.surface_nets_buffer.positions.is_empty()
            || self.surface_nets_buffer.indices.is_empty()
        {
            //减少组件生成
            self.mesh_sender.send((None, None, position)).unwrap();
            return;
        }

        let num_vertices = self.surface_nets_buffer.positions.len();

        let uvs = (0..num_vertices)
            .into_iter()
            .map(|i| {
                let material_id = chunk_data.sdf[i].material_id;
                [material_id as f32, 0.0]
            })
            .collect::<Vec<[f32; 2]>>();

        let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);
        render_mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            VertexAttributeValues::Float32x3(self.surface_nets_buffer.positions.clone()),
        );
        render_mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            VertexAttributeValues::Float32x3(self.surface_nets_buffer.normals.clone()),
        );
        render_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::Float32x2(uvs));
        render_mesh.set_indices(Some(Indices::U32(self.surface_nets_buffer.indices.clone())));

        let collider = Collider::trimesh_from_mesh(&render_mesh);

        let end = std::time::Instant::now();

        let duration = end - start;

        info!("chunk spawn duration: {:?}", duration);

        self.mesh_sender
            .send((Some(render_mesh), collider, position))
            .unwrap();
    }

    pub fn run(&mut self) {
        loop {
            match self.event_reciver.try_recv() {
                Ok(event) => match event {
                    VoxelEvent::Drop => {
                        break;
                    }
                    VoxelEvent::ChunkSpawn(p, e) => {
                        self.handle_chunk_spawn(p, e);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}
