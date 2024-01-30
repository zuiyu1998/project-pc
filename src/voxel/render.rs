use bevy::{asset::ReflectAsset, prelude::*, render::render_resource::AsBindGroup};

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[reflect(Asset)]
// #[uuid = "8014bf20-d959-11ed-afa1-0242ac120001"]
pub struct TerrainMaterial {
    #[sampler(0)]
    #[texture(1)]
    pub texture_diffuse: Option<Handle<Image>>,
    #[texture(2)]
    pub texture_normal: Option<Handle<Image>>,
    #[texture(3)]
    pub texture_dram: Option<Handle<Image>>,

    #[uniform(4)]
    pub sample_scale: f32,
    #[uniform(5)]
    pub normal_intensity: f32,
    #[uniform(6)]
    pub triplanar_blend_pow: f32,
    #[uniform(7)]
    pub heightmap_blend_pow: f32, // littler=mix, greater=distinct, opt 0.3 - 0.6, 0.48 = nature
}

impl Default for TerrainMaterial {
    fn default() -> Self {
        Self {
            texture_diffuse: None,
            texture_normal: None,
            texture_dram: None,
            sample_scale: 1.0,
            normal_intensity: 1.0,
            triplanar_blend_pow: 4.5,
            heightmap_blend_pow: 0.48,
        }
    }
}

impl Material for TerrainMaterial {
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/terrain.wgsl".into()
    }
    fn vertex_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/terrain.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}
