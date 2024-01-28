use bevy::prelude::*;
use bevy::{math::IVec3, utils::HashMap};
use ndshape::{ConstShape, ConstShape3u32};
use std::sync::{Arc, RwLock, Weak};

pub type MeshShape = ConstShape3u32<16, 16, 16>;

pub type ChunkPtr = Arc<RwLock<Chunk>>;

#[derive(Debug, Reflect, Default, Clone, Copy)]
pub struct SdfValue {
    pub value: f32,
}

impl SdfValue {
    pub fn is_empty(&self) -> bool {
        self.value <= 0.
    }

    pub fn new(value: f32) -> SdfValue {
        SdfValue { value }
    }
}

pub struct Chunk {
    pub position: IVec3,
    //实际存储的sdf值
    pub sdf: [SdfValue; MeshShape::USIZE],

    pub neighbor_chunks: HashMap<IVec3, Weak<RwLock<Chunk>>>,
}

impl Chunk {
    pub fn new(position: IVec3) -> Self {
        Self {
            position,
            sdf: [SdfValue::default(); MeshShape::USIZE],
            neighbor_chunks: Default::default(),
        }
    }
}

fn get_x(y: i32, k: i32) -> i32 {
    let mut is_positive = true;

    if y < 0 {
        is_positive = false;
    }

    let num = f32::ceil(y.abs() as f32 / k as f32) as i32;

    if is_positive {
        num * 1
    } else {
        num * -1
    }
}

impl Chunk {
    pub fn get_neighbor_position(&self, mesh_position: IVec3) -> (IVec3, IVec3) {
        let x = get_x(mesh_position.x, Chunk::MESH_X as i32);
        let y = get_x(mesh_position.y, Chunk::MESH_Y as i32);
        let z = get_x(mesh_position.z, Chunk::MESH_Z as i32);

        let relative = IVec3::new(x, y, z);

        let mesh_position = mesh_position
            - IVec3::new(
                Chunk::MESH_X as i32 * relative.x,
                Chunk::MESH_Y as i32 * relative.y,
                Chunk::MESH_Z as i32 * relative.z,
            );

        return (self.position + relative, mesh_position);
    }

    pub fn get_relative_position_sdf_value(&self, mesh_position: IVec3) -> Option<SdfValue> {
        let (position, mesh_position) = self.get_neighbor_position(mesh_position);

        if let Some(chunk_ptr) = self.neighbor_chunks.get(&position) {
            if let Some(chunk) = chunk_ptr.upgrade() {
                let chunk = chunk.read().unwrap();
                Some(chunk.get_location_position_sdf_value(mesh_position))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_location_position_sdf_value(&self, mesh_position: IVec3) -> SdfValue {
        let index = MeshShape::linearize([
            mesh_position.x as u32,
            mesh_position.z as u32,
            mesh_position.y as u32,
        ]);

        self.sdf[index as usize]
    }

    pub fn get_sdf_value(&self, mesh_position: IVec3) -> Option<SdfValue> {
        if Chunk::is_local_position(mesh_position) {
            Some(self.get_location_position_sdf_value(mesh_position))
        } else {
            self.get_relative_position_sdf_value(mesh_position)
        }
    }

    pub fn is_local_position(mesh_position: IVec3) -> bool {
        if !Self::is_loacal_x(mesh_position.x)
            || !Self::is_loacal_y(mesh_position.y)
            || !Self::is_loacal_z(mesh_position.z)
        {
            return false;
        } else {
            return true;
        }
    }

    pub fn is_loacal_x(x: i32) -> bool {
        if x < 0 || x >= Self::MESH_X as i32 {
            return false;
        } else {
            return true;
        }
    }

    pub fn is_loacal_y(y: i32) -> bool {
        if y < 0 || y >= Self::MESH_Y as i32 {
            return false;
        } else {
            return true;
        }
    }

    pub fn is_loacal_z(z: i32) -> bool {
        if z < 0 || z >= Self::MESH_Z as i32 {
            return false;
        } else {
            return true;
        }
    }

    pub const MESH_X: u32 = MeshShape::ARRAY[0];
    pub const MESH_Y: u32 = MeshShape::ARRAY[1];
    pub const MESH_Z: u32 = MeshShape::ARRAY[2];

    pub const NEIGHBOR_DIR: [IVec3; 6 + 12 + 8] = [
        // 6 Faces
        IVec3::new(-1, 0, 0),
        IVec3::new(1, 0, 0),
        IVec3::new(0, -1, 0),
        IVec3::new(0, 1, 0),
        IVec3::new(0, 0, -1),
        IVec3::new(0, 0, 1),
        // 12 Edges
        IVec3::new(0, -1, -1), // X
        IVec3::new(0, 1, 1),
        IVec3::new(0, 1, -1),
        IVec3::new(0, -1, 1),
        IVec3::new(-1, 0, -1), // Y
        IVec3::new(1, 0, 1),
        IVec3::new(1, 0, -1),
        IVec3::new(-1, 0, 1),
        IVec3::new(-1, -1, 0), // Z
        IVec3::new(1, 1, 0),
        IVec3::new(-1, 1, 0),
        IVec3::new(1, -1, 0),
        // 8 Vertices
        IVec3::new(-1, -1, -1),
        IVec3::new(1, 1, 1),
        IVec3::new(1, -1, -1),
        IVec3::new(-1, 1, 1),
        IVec3::new(-1, -1, 1),
        IVec3::new(1, 1, -1),
        IVec3::new(1, -1, 1),
        IVec3::new(-1, 1, -1),
    ];
}

mod test {

    #[test]
    pub fn test_is_local_position() {
        use super::Chunk;
        use bevy::math::IVec3;

        let p1 = IVec3::new(16, 15, 15);
        assert_eq!(Chunk::is_local_position(p1), false);

        let p2: IVec3 = IVec3::new(-1, 15, 15);
        assert_eq!(Chunk::is_local_position(p2), false);

        let p3: IVec3 = IVec3::new(15, 15, 15);
        assert_eq!(Chunk::is_local_position(p3), true);
    }

    #[test]
    pub fn test_get_x() {
        use super::get_x;

        assert_eq!(get_x(-1, 16), -1);
        assert_eq!(get_x(16, 16), 1);
        assert_eq!(get_x(0, 16), 0);
    }
}
