mod chunk;
mod surface_nets_helper;

use std::sync::{Arc, RwLock};

use self::surface_nets_helper::{SurfaceNetsBuffer, SurfaceNetsHelper};
use bevy::{
    prelude::*,
    render::mesh::{Indices, VertexAttributeValues},
};
pub use chunk::*;
use ndshape::ConstShape;

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

#[derive(Debug, Hash, PartialEq, Eq, Reflect, Deref, DerefMut, Component, Clone)]
pub struct ChunkPosition(IVec3);

impl ChunkPosition {}

fn setup_voxel(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands
        .spawn_empty()
        .insert(Name::new("Chunks"))
        .insert((TransformBundle::default(), VisibilityBundle::default()));

    let mut chunk = Chunk::new(IVec3::new(0, 0, 0));

    for i in 0u32..MeshShape::SIZE {
        let [x, y, z] = MeshShape::delinearize(i);

        let value = ((x * x + y * y + z * z) as f32).sqrt() - 15.0;

        chunk.sdf[i as usize] = SdfValue::new(value);
    }

    let mut buffer = SurfaceNetsBuffer::default();

    buffer.reset();

    let chunk_ptr = Arc::new(RwLock::new(chunk));

    let helper = SurfaceNetsHelper::new(chunk_ptr);

    helper.surface_nets(&mut buffer);

    let num_vertices = buffer.positions.len();

    let mut render_mesh = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList);
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

    let mesh = meshes.add(render_mesh);

    let mut material = StandardMaterial::from(Color::rgb(0.0, 0.0, 0.0));
    material.perceptual_roughness = 0.9;

    commands.spawn(PbrBundle {
        mesh,
        material: materials.add(material),
        ..Default::default()
    });
}

pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_voxel,));
    }
}
