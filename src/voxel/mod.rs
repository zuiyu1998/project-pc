mod map;
mod material;
mod render;
mod sdf_value;

use map::*;
use material::*;
use render::*;
use sdf_value::*;

use bevy::prelude::*;
use bevy_xpbd_3d::components::RigidBody;
use std::cmp::Ord;

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

fn setup_voxel(
    mut commands: Commands,
    mut mesh_reciver: NonSendMut<MeshReceiverResource>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
    mut map: ResMut<Map>,
    asset_server: Res<AssetServer>,
) {
    let mtl = terrain_materials.add(TerrainMaterial {
        texture_diffuse: Some(asset_server.load("cache/atlas_diff.png")),
        texture_normal: Some(asset_server.load("cache/atlas_norm.png")),
        texture_dram: Some(asset_server.load("cache/atlas_dram.png")),
        ..default()
    });

    map.voxel_terrain_material = mtl;

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

    positions.sort_by(|a, b| {
        let a = a.0.length_squared();
        let b = b.0.length_squared();
        a.cmp(&b)
    });

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
            Ok((mesh, c, p)) => {
                if mesh.is_none() {
                    if let Some(_) = map.loading_meshs.get(&p) {
                        map.loading_meshs.remove(&p);
                    }

                    return;
                }

                let mesh = mesh.unwrap();
                let c = c.unwrap();

                mesh_cache.push((mesh, c, p));
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
    mut mesh_cache: ResMut<MeshCache>,
) {
    let events = mesh_cache.pop();

    if events.is_none() {
        return;
    }

    let events = events.unwrap();

    for (mesh, c, p) in events.into_iter() {
        if let Some(entity) = map.meshs.get(&p) {
            let mesh = meshes.add(mesh);

            commands.entity(*entity).insert((
                MaterialMeshBundle {
                    mesh,
                    material: map.voxel_terrain_material.clone(),
                    transform: Transform::from_translation(p.as_vec3() * VoxelData::MESH as f32),

                    ..default()
                },
                RigidBody::Static,
                c,
            ));

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
            .register_type::<Map>()
            .add_plugins(MaterialPlugin::<TerrainMaterial>::default())
            .register_asset_reflect::<TerrainMaterial>();
    }
}
