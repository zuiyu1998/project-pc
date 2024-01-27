mod surface_nets_helper;

use std::ops::Div;

use bevy::{
    ecs::system::{CommandQueue, SystemState},
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
    tasks::{block_on, AsyncComputeTaskPool, Task},
};

use futures_lite::future;

use ndshape::{ConstShape, ConstShape3u32};
use noise::{Fbm, NoiseFn, Perlin};

use crate::voxel::surface_nets_helper::{SurfaceNetsBuffer, SurfaceNetsHelper};

pub type MeshShape = ConstShape3u32<16, 16, 16>;

/*
理想状态:
1.获取一个高度图
2.从高度图中获取sdf
3.从sdf中获取mesh
工程实现:
1.获取一个高度图
2.将sdf以区块划分
3.生成mesh的时候，依赖当前区块和邻近区块生成mesh.如果区块的mesh未存在，则标记区块，同时标记需要重新生成的mesh
*/

#[derive(Debug, Reflect, Default, Clone, Copy)]
pub struct SdfValue {
    pub value: f32,
}

impl SdfValue {
    pub fn is_empty(&self) -> bool {
        self.value <= 0.
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Reflect, Deref, DerefMut, Component, Clone)]
pub struct ChunkPosition(IVec3);

impl ChunkPosition {
    pub fn get_relative_position(&self, mesh_position: IVec3) -> IVec3 {
        mesh_position
            + IVec3::new(
                self.x * MeshShape::ARRAY[0] as i32,
                self.y * MeshShape::ARRAY[1] as i32,
                self.z * MeshShape::ARRAY[2] as i32,
            )
    }
}

#[derive(Debug, Reflect, Deref, DerefMut)]
pub struct SdfBuffer([SdfValue; MeshShape::USIZE]);

impl Default for SdfBuffer {
    fn default() -> Self {
        SdfBuffer([SdfValue::default(); MeshShape::USIZE])
    }
}

#[derive(Debug, Reflect)]
pub struct PerlinNoise {
    seed: u32,
}

impl Default for PerlinNoise {
    fn default() -> Self {
        PerlinNoise { seed: 100 }
    }
}

#[derive(Reflect, Resource, Default)]
pub struct Map {
    noise: PerlinNoise,
}

impl Map {
    pub fn get_sdf_value(seed: u32, position: IVec3) -> SdfValue {
        let mut fbm = Fbm::<Perlin>::new(seed);

        fbm.octaves = 4;

        let f_terr = fbm.get(position.xz().as_dvec2().div(129.).to_array()) as f32;
        let f_3d = fbm.get(position.as_dvec3().div(70.).to_array()) as f32;

        let val = f_terr - (position.y as f32) / 12. + f_3d * 2.5;

        SdfValue { value: val }
    }
}

#[derive(Component)]
pub struct Chunks;

#[derive(Resource)]
pub struct SpawnChunks(Vec<ChunkPosition>);

#[derive(Component)]
pub struct SpawnChunkTasks(Task<CommandQueue>);

fn spawn_chunks(mut commands: Commands, spawn_chunks: Option<Res<SpawnChunks>>) {
    if spawn_chunks.is_none() {
        return;
    }
    let spawn_chunks = spawn_chunks.unwrap();

    let thread_pool = AsyncComputeTaskPool::get();

    for chunk in spawn_chunks.0.iter() {
        let position = (*chunk).clone();
        let chunk = (*chunk).clone();
        let entity = commands.spawn_empty().id();

        let task = thread_pool.spawn(async move {
            let transform = Transform::from_xyz(
                chunk.x as f32 * MeshShape::ARRAY[0] as f32,
                chunk.y as f32 * MeshShape::ARRAY[1] as f32,
                chunk.z as f32 * MeshShape::ARRAY[2] as f32,
            );

            let mut command_queue = CommandQueue::default();

            command_queue.push(move |world: &mut World| {
                let (mesh,) = {
                    let mut system_state =
                        SystemState::<(ResMut<Map>, ResMut<Assets<Mesh>>)>::new(world);
                    let (map, mut meshs) = system_state.get_mut(world);

                    let mut helper = SurfaceNetsHelper::new(&map, &chunk);

                    let mut buffer = SurfaceNetsBuffer::default();

                    buffer.reset();

                    helper.surface_nets(&mut buffer);
                    let num_vertices = buffer.positions.len();

                    let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);
                    render_mesh.insert_attribute(
                        Mesh::ATTRIBUTE_POSITION,
                        VertexAttributeValues::Float32x3(buffer.positions.clone()),
                    );
                    render_mesh.insert_attribute(
                        Mesh::ATTRIBUTE_NORMAL,
                        VertexAttributeValues::Float32x3(buffer.normals.clone()),
                    );
                    render_mesh.insert_attribute(
                        Mesh::ATTRIBUTE_UV_0,
                        VertexAttributeValues::Float32x2(vec![[0.0; 2]; num_vertices]),
                    );
                    render_mesh.set_indices(Some(Indices::U32(buffer.indices.clone())));

                    let mesh = meshs.add(render_mesh);

                    (mesh,)
                };

                world
                    .entity_mut(entity)
                    // Add our new PbrBundle of components to our tagged entity
                    .insert(PbrBundle {
                        mesh,
                        // material: box_material_handle,
                        transform,
                        ..default()
                    })
                    // Task is complete, so remove task component from entity
                    .remove::<SpawnChunkTasks>();
            });

            command_queue
        });

        commands
            .entity(entity)
            .insert(SpawnChunkTasks(task))
            .insert(position);
    }

    commands.remove_resource::<SpawnChunks>();
}

fn handle_spawn_tasks(world: &mut World) {
    let mut transform_tasks = world.query::<&mut SpawnChunkTasks>();

    let mut commands_queue_optional: Option<CommandQueue> = None;

    for mut task in transform_tasks.iter_mut(world) {
        if let Some(commands_queue) = block_on(future::poll_once(&mut task.0)) {
            commands_queue_optional = Some(commands_queue);
        }
    }

    if let Some(mut commands_queue) = commands_queue_optional {
        commands_queue.apply(world);
    }
}
fn setup_voxel(mut commands: Commands) {
    commands
        .spawn_empty()
        .insert(Name::new("Chunks"))
        .insert(Chunks)
        .insert(TransformBundle::default());
}

pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_voxel,))
            .add_systems(Update, (spawn_chunks, handle_spawn_tasks))
            .insert_resource(Map::default());
    }
}
