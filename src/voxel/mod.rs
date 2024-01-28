//mod surface_nets_helper;

mod chunk;

use bevy::prelude::*;

pub use chunk::*;

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

fn setup_voxel(mut commands: Commands) {
    commands
        .spawn_empty()
        .insert(Name::new("Chunks"))
        .insert((TransformBundle::default(), VisibilityBundle::default()));
}

pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_voxel,));
    }
}
