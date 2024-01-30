pub struct VoxelMaterial {
    hardness: f32,

    /// Foliage/Vegetable Materials will generate to another mesh., with Double-Sided (NoCulling), NoCollision, WavingVertex Rendering
    is_foliage: bool,
    // custom_mesh
    // tex_id

    // item: Rc<Item>
}

impl VoxelMaterial {
    pub const STONE: u16 = 21;
    pub const DIRT: u16 = 0;
    pub const GRASS: u16 = 11; // 7 11
    pub const WATER: u16 = 23;
    pub const SAND: u16 = 18;
}

impl Default for VoxelMaterial {
    fn default() -> Self {
        Self {
            hardness: 1.,
            is_foliage: false,
        }
    }
}
