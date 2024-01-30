mod map;

use map::*;

use bevy::prelude::*;

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

fn setup_voxel(mut commands: Commands, mut mesh_reciver: NonSendMut<MeshReceiverResource>) {
    let (event_sender_resource, receiver_resource, handle) = start_handle_voxel_data_thread();

    *mesh_reciver = receiver_resource;

    commands.insert_resource(event_sender_resource);
    commands.insert_resource(handle);
}

pub fn spawn_mesh(
    mut commands: Commands,
    mut map: ResMut<Map>,
    voxel_event_sender: Res<VoxelEventSenderResource>,
    spawn_meshs: Option<ResMut<SpawnMeshs>>,
) {
    if spawn_meshs.is_none() {
        return;
    }

    let spawn_meshs = spawn_meshs.unwrap();

    let mut positions = vec![];

    spawn_meshs.iter().for_each(|p| {
        if let None = map.meshs.get(p) {
            positions.push(p.to_owned());
        }
    });

    if positions.is_empty() {
        commands.remove_resource::<SpawnMeshs>();

        return;
    }

    info!("positions len: {}", positions.len());

    for p in positions.iter() {
        let p = p.to_owned();

        let entity = commands.spawn_empty().id();

        voxel_event_sender
            .send(VoxelEvent::ChunkSpawn(p.to_owned(), entity))
            .unwrap();

        map.loading_meshs.insert(p.to_owned(), entity);
        map.meshs.insert(p.to_owned(), entity);

        commands.entity(entity).insert(p);
    }
}

pub fn receive_mesh(
    mesh_reciver: NonSend<MeshReceiverResource>,
    mut map: ResMut<Map>,
    mut mesh_cache: ResMut<MeshCache>,
) {
    if let Some(mesh_reciver) = &(mesh_reciver.0) {
        match mesh_reciver.try_recv() {
            Ok((mesh, p)) => {
                if mesh.is_none() {
                    if let Some(_) = map.loading_meshs.get(&p) {
                        map.loading_meshs.remove(&p);
                    }

                    return;
                }

                let mesh = mesh.unwrap();

                mesh_cache.push((mesh, p));
            }
            Err(_) => {}
        }
    }
}

//削峰
pub fn processing_meshs(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut map: ResMut<Map>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mesh_cache: ResMut<MeshCache>,
) {
    let events = mesh_cache.pop();

    if events.is_none() {
        return;
    }

    let events = events.unwrap();

    for (mesh, p) in events.into_iter() {
        if let Some(entity) = map.meshs.get(&p) {
            let mut material = StandardMaterial::from(Color::rgb(0.0, 0.0, 0.0));
            material.perceptual_roughness = 0.9;

            let mesh = meshes.add(mesh);

            commands.entity(*entity).insert(PbrBundle {
                mesh,
                material: materials.add(material),
                transform: Transform {
                    translation: p.as_vec3(),
                    ..Default::default()
                },
                ..Default::default()
            });

            map.loading_meshs.remove(&p);
        }
    }
}

pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_voxel,))
            .add_systems(Update, (spawn_mesh, receive_mesh, processing_meshs))
            .register_type::<ChunkPosition>()
            .insert_non_send_resource(MeshReceiverResource::default())
            .init_resource::<SpawnMeshs>()
            .init_resource::<MeshCache>()
            .insert_resource(Map {
                ..Default::default()
            })
            .register_type::<Map>();
    }
}
