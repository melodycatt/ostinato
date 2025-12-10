#[cfg(feature = "custom_vertex")]
use std::collections::HashMap;
#[cfg(feature = "custom_vertex")]
use std::sync::Mutex;

#[cfg(not(feature = "custom_vertex"))]
use anyhow::anyhow;

#[cfg(feature = "custom_vertex")]
type DescFn = fn() -> wgpu::VertexBufferLayout<'static>;

#[cfg(feature = "custom_vertex")]
lazy_static::lazy_static! {
    static ref REGISTRY: Mutex<HashMap<String, DescFn>> = Mutex::new(HashMap::new());
}

#[cfg(feature = "custom_vertex")]
pub fn register_type(name: &str, desc: DescFn) {
    REGISTRY.lock().unwrap().insert(name.to_string(), desc);
}

pub fn desc_from_name(name: &str) -> anyhow::Result<wgpu::VertexBufferLayout<'static>> {
    match name {
        "COLOR_VERTEX" => Ok(ColorVertex::desc()),
        "TEXTURE_VERTEX" => Ok(TextureVertex::desc()),
        "MODEL_VERTEX" => Ok(ModelVertex::desc()),
        _ => {
            #[cfg(feature = "custom_vertex")]
            return Ok(REGISTRY.lock().map_err(|_| anyhow::anyhow!("x_x :: tried to access poisoned or locked vertex registry when loading material (this will only be your fault, ostinato never uses the vertex registry)"))?
                    .get(name).ok_or(anyhow::anyhow!("x_x :: unregistered vertex type when loading material: {}", name))?());
            #[cfg(not(feature = "custom_vertex"))]
            return Err(anyhow!("x_x :: invalid vertex type when loading material: {}", name));
        }
    }
}

pub trait Vertex : bytemuck::Pod+bytemuck::Zeroable+'static { const TYPE: VertexType; fn desc() -> wgpu::VertexBufferLayout<'static>; }

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ColorVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl Vertex for ColorVertex {
    const TYPE: VertexType = VertexType::Color;
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ColorVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextureVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl Vertex for TextureVertex {
    const TYPE: VertexType = VertexType::Texture;
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<TextureVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl Vertex for ModelVertex {
    const TYPE: VertexType = VertexType::Model;
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub enum VertexType {
    Color,
    Texture,
    Model,
}