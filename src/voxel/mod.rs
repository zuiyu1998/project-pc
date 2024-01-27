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

use fast_surface_nets::ndshape::{ConstShape, ConstShape3u32};
use fast_surface_nets::{surface_nets, SurfaceNetsBuffer};
use noise::{Fbm, NoiseFn, Perlin};

pub type ChunkShape = ConstShape3u32<16, 16, 16>;

pub type MeshShape = ConstShape3u32<16, 16, 16>;

#[derive(Resource)]
pub struct LoadedChunks(pub Option<Vec<u32>>);

#[derive(Component)]
struct SpawnChunkTask(Task<CommandQueue>);

#[derive(Component)]
pub struct Chunks;

#[derive(Component)]
pub struct Chunk {
    buffer: SurfaceNetsBuffer,
    sdf: [f32; ChunkShape::USIZE],
    position: u32,
}

impl Chunk {
    pub fn new(position: u32) -> Chunk {
        let seed = 100;
        let mut fbm = Fbm::<Perlin>::new(seed);
        fbm.octaves = 4;

        let mut sdf = [1.0; MeshShape::USIZE];
        for i in 0u32..MeshShape::SIZE {
            let [x, y, z] = MeshShape::delinearize(i);
            let [px, py, pz] = ChunkShape::delinearize(position);

            let position = Vec3::new((x + px) as f32, (py + y) as f32, (pz + z) as f32);

            let f_terr = fbm.get(position.xz().as_dvec2().div(129.).to_array()) as f32;
            let f_3d = fbm.get(position.as_dvec3().div(70.).to_array()) as f32;

            let val = f_terr - (position.y as f32) / 12. + f_3d * 2.5;

            sdf[i as usize] = val;
        }

        let mut buffer = SurfaceNetsBuffer::default();
        surface_nets(&sdf, &MeshShape {}, [0; 3], [15; 3], &mut buffer);

        Chunk {
            buffer,
            sdf,
            position,
        }
    }

    pub fn to_mesh(&self) -> Mesh {
        let num_vertices = self.buffer.positions.len();

        let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);
        render_mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            VertexAttributeValues::Float32x3(self.buffer.positions.clone()),
        );
        render_mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            VertexAttributeValues::Float32x3(self.buffer.normals.clone()),
        );
        render_mesh.insert_attribute(
            Mesh::ATTRIBUTE_UV_0,
            VertexAttributeValues::Float32x2(vec![[0.0; 2]; num_vertices]),
        );
        render_mesh.set_indices(Some(Indices::U32(self.buffer.indices.clone())));

        render_mesh
    }
}

fn spawn_chunks(
    mut commands: Commands,
    mut loaded_chunks: ResMut<LoadedChunks>,
    chunks: Query<Entity, With<Chunks>>,
) {
    let chunks_entity = chunks.single();
    let thread_pool = AsyncComputeTaskPool::get();
    if let Some(postions) = loaded_chunks.0.take() {
        for position in postions.into_iter() {
            let entity = commands.spawn_empty().id();
            let task = thread_pool.spawn(async move {
                let mut command_queue = CommandQueue::default();

                // we use a raw command queue to pass a FnOne(&mut World) back to be
                // applied in a deferred manner.
                command_queue.push(move |world: &mut World| {
                    let chunk = Chunk::new(position);

                    let [px, py, pz] = ChunkShape::delinearize(position);

                    let position_normal =
                        Vec3::new(px as f32 - 8.0, py as f32 - 8.0, pz as f32 - 8.0);

                    let transform = Transform {
                        translation: position_normal * 16.0,
                        ..Default::default()
                    };

                    let (mesh, material) = {
                        let mut system_state = SystemState::<(
                            ResMut<Assets<Mesh>>,
                            ResMut<Assets<StandardMaterial>>,
                        )>::new(world);
                        let (mut mesh_assets, mut material_assets) = system_state.get_mut(world);

                        let mesh = chunk.to_mesh();
                        let mesh = mesh_assets.add(mesh);

                        let mut material = StandardMaterial::from(Color::rgb(1.0, 1.0, 0.0));
                        material.perceptual_roughness = 0.9;

                        let material = material_assets.add(material);

                        (mesh, material)
                    };

                    world
                        .entity_mut(entity)
                        .insert(PbrBundle {
                            mesh,
                            material,
                            transform,
                            ..Default::default()
                        })
                        .insert(chunk)
                        .remove::<SpawnChunkTask>();
                });

                command_queue
            });

            // Spawn new entity and add our new task as a component
            commands
                .entity(entity)
                .insert(SpawnChunkTask(task))
                .set_parent(chunks_entity);
        }
    }
}

fn handle_spawn_chunk_tasks(world: &mut World) {
    let mut transform_tasks = world.query::<&mut SpawnChunkTask>();

    let mut commands_queues = vec![];

    for mut task in transform_tasks.iter_mut(world) {
        if let Some(commands_queue) = block_on(future::poll_once(&mut task.0)) {
            commands_queues.push(commands_queue);
        }
    }

    for mut commands_queue in commands_queues.into_iter() {
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
        app.insert_resource(LoadedChunks(Some(vec![0, 20, 15, 30])));
        app.add_systems(Update, (spawn_chunks, handle_spawn_chunk_tasks));
        app.add_systems(Startup, (setup_voxel,));
    }
}
