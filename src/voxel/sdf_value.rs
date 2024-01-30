use super::VoxelMaterial;
use fast_surface_nets::SignedDistance;

#[derive(Clone, Copy)]
pub struct SdfValue {
    pub value: f32,
    pub material_id: u16,
}

impl SdfValue {
    pub fn new(value: f32, material_id: u16) -> Self {
        SdfValue { value, material_id }
    }
}

impl Default for SdfValue {
    fn default() -> Self {
        SdfValue {
            value: 1.0,
            material_id: VoxelMaterial::STONE,
        }
    }
}

impl Into<f32> for SdfValue {
    fn into(self) -> f32 {
        self.value
    }
}

impl SignedDistance for SdfValue {
    fn is_negative(self) -> bool {
        self.value < 0.0
    }
}
