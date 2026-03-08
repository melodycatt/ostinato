#[cfg(feature = "custom_vertex")]
use std::collections::HashMap;
use std::fmt::Debug;
#[cfg(feature = "custom_vertex")]
use std::sync::Mutex;

#[cfg(not(feature = "custom_vertex"))]
use anyhow::anyhow;
use bytemuck::{Pod, Zeroable};
use wgpu::vertex_attr_array;

use crate::renderer::InstanceRaw;

#[cfg(feature = "custom_vertex")]
type BufferLayoutFn = fn(u32) -> BufferLayout;

#[cfg(feature = "custom_vertex")]
lazy_static::lazy_static! {
    static ref REGISTRY: Mutex<HashMap<String, BufferLayoutFn>> = Mutex::new(HashMap::new());
}

#[cfg(feature = "custom_vertex")]
pub fn register_type<T: VertexBuffer>(name: &str) {
    REGISTRY
        .lock()
        .unwrap()
        .insert(name.to_string(), T::buffer_layout);
}

pub struct BufferLayout {
    pub attrs: Vec<wgpu::VertexAttribute>,
    pub stride: u64,
    pub step_mode: wgpu::VertexStepMode,
}

pub fn vertex_from_name(name: &str, location: u32) -> anyhow::Result<BufferLayout> {
    match name {
        "ColorVertex" => Ok(ColorVertex::buffer_layout(location)),
        "TextureVertex" => Ok(TextureVertex::buffer_layout(location)),
        "ModelVertex" => Ok(ModelVertex::buffer_layout(location)),
        //"Instance" => Ok((InstanceRaw::attrs, InstanceRaw::desc)),
        // Float formats

        // Float formats
        "Float32x2" => Ok(<SimpleVertex<[f32; 2], StepVertex>>::buffer_layout(
            location,
        )),
        "Float32x3" => Ok(<SimpleVertex<[f32; 3], StepVertex>>::buffer_layout(
            location,
        )),
        "Float32x4" => Ok(<SimpleVertex<[f32; 4], StepVertex>>::buffer_layout(
            location,
        )),

        // Unsigned 8-bit integer
        "Uint8" => Ok(<SimpleVertex<u8, StepVertex>>::buffer_layout(location)),
        "Uint8x2" => Ok(<SimpleVertex<[u8; 2], StepVertex>>::buffer_layout(location)),
        "Uint8x4" => Ok(<SimpleVertex<[u8; 4], StepVertex>>::buffer_layout(location)),

        // Signed 8-bit integer
        "Sint8" => Ok(<SimpleVertex<i8, StepVertex>>::buffer_layout(location)),
        "Sint8x2" => Ok(<SimpleVertex<[i8; 2], StepVertex>>::buffer_layout(location)),
        "Sint8x4" => Ok(<SimpleVertex<[i8; 4], StepVertex>>::buffer_layout(location)),

        // Normalized unsigned 8-bit
        "Unorm8x2" => Ok(<SimpleVertex<[Unorm8; 2], StepVertex>>::buffer_layout(
            location,
        )),
        "Unorm8x4" => Ok(<SimpleVertex<[Unorm8; 4], StepVertex>>::buffer_layout(
            location,
        )),

        // Normalized signed 8-bit
        "Snorm8x2" => Ok(<SimpleVertex<[Snorm8; 2], StepVertex>>::buffer_layout(
            location,
        )),
        "Snorm8x4" => Ok(<SimpleVertex<[Snorm8; 4], StepVertex>>::buffer_layout(
            location,
        )),

        // Unsigned 16-bit integer
        "Uint16" => Ok(<SimpleVertex<u16, StepVertex>>::buffer_layout(location)),
        "Uint16x2" => Ok(<SimpleVertex<[u16; 2], StepVertex>>::buffer_layout(
            location,
        )),
        "Uint16x4" => Ok(<SimpleVertex<[u16; 4], StepVertex>>::buffer_layout(
            location,
        )),

        // Signed 16-bit integer
        "Sint16" => Ok(<SimpleVertex<i16, StepVertex>>::buffer_layout(location)),
        "Sint16x2" => Ok(<SimpleVertex<[i16; 2], StepVertex>>::buffer_layout(
            location,
        )),
        "Sint16x4" => Ok(<SimpleVertex<[i16; 4], StepVertex>>::buffer_layout(
            location,
        )),

        // Normalized unsigned 16-bit
        "Unorm16x2" => Ok(<SimpleVertex<[Unorm16; 2], StepVertex>>::buffer_layout(
            location,
        )),
        "Unorm16x4" => Ok(<SimpleVertex<[Unorm16; 4], StepVertex>>::buffer_layout(
            location,
        )),

        // Normalized signed 16-bit
        "Snorm16x2" => Ok(<SimpleVertex<[Snorm16; 2], StepVertex>>::buffer_layout(
            location,
        )),
        "Snorm16x4" => Ok(<SimpleVertex<[Snorm16; 4], StepVertex>>::buffer_layout(
            location,
        )),

        // Unsigned 32-bit integer
        "Uint32" => Ok(<SimpleVertex<u32, StepVertex>>::buffer_layout(location)),
        "Uint32x2" => Ok(<SimpleVertex<[u32; 2], StepVertex>>::buffer_layout(
            location,
        )),
        "Uint32x3" => Ok(<SimpleVertex<[u32; 3], StepVertex>>::buffer_layout(
            location,
        )),
        "Uint32x4" => Ok(<SimpleVertex<[u32; 4], StepVertex>>::buffer_layout(
            location,
        )),

        // Signed 32-bit integer
        "Sint32" => Ok(<SimpleVertex<i32, StepVertex>>::buffer_layout(location)),
        "Sint32x2" => Ok(<SimpleVertex<[i32; 2], StepVertex>>::buffer_layout(
            location,
        )),
        "Sint32x3" => Ok(<SimpleVertex<[i32; 3], StepVertex>>::buffer_layout(
            location,
        )),
        "Sint32x4" => Ok(<SimpleVertex<[i32; 4], StepVertex>>::buffer_layout(
            location,
        )),
        _ => {
            #[cfg(feature = "custom_vertex")]
            return Ok(REGISTRY.lock().map_err(|_| anyhow::anyhow!("x_x :: tried to access poisoned or locked vertex registry when loading material (this will only be your fault, ostinato never uses the vertex registry)"))?
                    .get(name).ok_or(anyhow::anyhow!("x_x :: unregistered vertex type when loading material: {}", name))?());
            #[cfg(not(feature = "custom_vertex"))]
            return Err(anyhow!(
                "x_x :: invalid vertex type when loading material: {}",
                name
            ));
        }
    }
}
pub fn instance_from_name(name: &str, location: u32) -> anyhow::Result<BufferLayout> {
    match name {
        "Instance" => Ok(InstanceRaw::buffer_layout(location)),
        //
        // Float formats

        // Float formats
        "Float32x2" => Ok(<SimpleVertex<[f32; 2], StepInstance>>::buffer_layout(
            location,
        )),
        "Float32x3" => Ok(<SimpleVertex<[f32; 3], StepInstance>>::buffer_layout(
            location,
        )),
        "Float32x4" => Ok(<SimpleVertex<[f32; 4], StepInstance>>::buffer_layout(
            location,
        )),

        // Unsigned 8-bit integer
        "Uint8" => Ok(<SimpleVertex<u8, StepInstance>>::buffer_layout(location)),
        "Uint8x2" => Ok(<SimpleVertex<[u8; 2], StepInstance>>::buffer_layout(
            location,
        )),
        "Uint8x4" => Ok(<SimpleVertex<[u8; 4], StepInstance>>::buffer_layout(
            location,
        )),

        // Signed 8-bit integer
        "Sint8" => Ok(<SimpleVertex<i8, StepInstance>>::buffer_layout(location)),
        "Sint8x2" => Ok(<SimpleVertex<[i8; 2], StepInstance>>::buffer_layout(
            location,
        )),
        "Sint8x4" => Ok(<SimpleVertex<[i8; 4], StepInstance>>::buffer_layout(
            location,
        )),

        // Normalized unsigned 8-bit
        "Unorm8x2" => Ok(<SimpleVertex<[Unorm8; 2], StepInstance>>::buffer_layout(
            location,
        )),
        "Unorm8x4" => Ok(<SimpleVertex<[Unorm8; 4], StepInstance>>::buffer_layout(
            location,
        )),

        // Normalized signed 8-bit
        "Snorm8x2" => Ok(<SimpleVertex<[Snorm8; 2], StepInstance>>::buffer_layout(
            location,
        )),
        "Snorm8x4" => Ok(<SimpleVertex<[Snorm8; 4], StepInstance>>::buffer_layout(
            location,
        )),

        // Unsigned 16-bit integer
        "Uint16" => Ok(<SimpleVertex<u16, StepInstance>>::buffer_layout(location)),
        "Uint16x2" => Ok(<SimpleVertex<[u16; 2], StepInstance>>::buffer_layout(
            location,
        )),
        "Uint16x4" => Ok(<SimpleVertex<[u16; 4], StepInstance>>::buffer_layout(
            location,
        )),

        // Signed 16-bit integer
        "Sint16" => Ok(<SimpleVertex<i16, StepInstance>>::buffer_layout(location)),
        "Sint16x2" => Ok(<SimpleVertex<[i16; 2], StepInstance>>::buffer_layout(
            location,
        )),
        "Sint16x4" => Ok(<SimpleVertex<[i16; 4], StepInstance>>::buffer_layout(
            location,
        )),

        // Normalized unsigned 16-bit
        "Unorm16x2" => Ok(<SimpleVertex<[Unorm16; 2], StepInstance>>::buffer_layout(
            location,
        )),
        "Unorm16x4" => Ok(<SimpleVertex<[Unorm16; 4], StepInstance>>::buffer_layout(
            location,
        )),

        // Normalized signed 16-bit
        "Snorm16x2" => Ok(<SimpleVertex<[Snorm16; 2], StepInstance>>::buffer_layout(
            location,
        )),
        "Snorm16x4" => Ok(<SimpleVertex<[Snorm16; 4], StepInstance>>::buffer_layout(
            location,
        )),

        // Unsigned 32-bit integer
        "Uint32" => Ok(<SimpleVertex<u32, StepInstance>>::buffer_layout(location)),
        "Uint32x2" => Ok(<SimpleVertex<[u32; 2], StepInstance>>::buffer_layout(
            location,
        )),
        "Uint32x3" => Ok(<SimpleVertex<[u32; 3], StepInstance>>::buffer_layout(
            location,
        )),
        "Uint32x4" => Ok(<SimpleVertex<[u32; 4], StepInstance>>::buffer_layout(
            location,
        )),

        // Signed 32-bit integer
        "Sint32" => Ok(<SimpleVertex<i32, StepInstance>>::buffer_layout(location)),
        "Sint32x2" => Ok(<SimpleVertex<[i32; 2], StepInstance>>::buffer_layout(
            location,
        )),
        "Sint32x3" => Ok(<SimpleVertex<[i32; 3], StepInstance>>::buffer_layout(
            location,
        )),
        "Sint32x4" => Ok(<SimpleVertex<[i32; 4], StepInstance>>::buffer_layout(
            location,
        )),
        _ => {
            #[cfg(feature = "custom_vertex")]
            return Ok(REGISTRY.lock().map_err(|_| anyhow::anyhow!("x_x :: tried to access poisoned or locked vertex registry when loading material (this will only be your fault, ostinato never uses the vertex registry)"))?
                    .get(name).ok_or(anyhow::anyhow!("x_x :: unregistered vertex type when loading material: {}", name))?());
            #[cfg(not(feature = "custom_vertex"))]
            return Err(anyhow!(
                "x_x :: invalid vertex type when loading material: {}",
                name
            ));
        }
    }
}

pub trait VertexBuffer: bytemuck::Pod + bytemuck::Zeroable + 'static {
    const STRIDE: u32;
    const STEP_MODE: wgpu::VertexStepMode = wgpu::VertexStepMode::Vertex;
    fn buffer_layout(location: u32) -> BufferLayout {
        BufferLayout {
            attrs: Self::attrs(location),
            stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: Self::STEP_MODE,
        }
    }
    //fn desc(vertex_attrs: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout<'_>;
    fn attrs(location: u32) -> Vec<wgpu::VertexAttribute>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ColorVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl VertexBuffer for ColorVertex {
    const STRIDE: u32 = 2;
    // fn desc(vertex_attrs: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout<'_> {
    //     use std::mem;
    //     wgpu::VertexBufferLayout {
    //         array_stride: mem::size_of::<ColorVertex>() as wgpu::BufferAddress,
    //         step_mode: wgpu::VertexStepMode::Vertex,
    //         attributes: vertex_attrs,
    //     }
    // }
    fn attrs(location: u32) -> Vec<wgpu::VertexAttribute> {
        wgpu::vertex_attr_array![
            location => Float32x3,
            location + 1 => Float32x2
        ]
        .to_vec()
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextureVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl VertexBuffer for TextureVertex {
    const STRIDE: u32 = 2;
    // fn desc(vertex_attrs: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout<'_> {
    //     use std::mem;
    //     wgpu::VertexBufferLayout {
    //         array_stride: mem::size_of::<TextureVertex>() as wgpu::BufferAddress,
    //         step_mode: wgpu::VertexStepMode::Vertex,
    //         attributes: vertex_attrs,
    //     }
    // }
    fn attrs(location: u32) -> Vec<wgpu::VertexAttribute> {
        vertex_attr_array![
            location => Float32x3,
            location + 1 => Float32x2
        ]
        .to_vec()
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl VertexBuffer for ModelVertex {
    const STRIDE: u32 = 3;
    // fn desc(vertex_attrs: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout<'_> {
    //     use std::mem;
    //     wgpu::VertexBufferLayout {
    //         array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
    //         step_mode: wgpu::VertexStepMode::Vertex,
    //         attributes: vertex_attrs,
    //     }
    // }
    fn attrs(location: u32) -> Vec<wgpu::VertexAttribute> {
        vertex_attr_array![
            location => Float32x3,
            location + 1 => Float32x2,
            location + 2 => Float32x3
        ]
        .to_vec()
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimpleVertex<T: VertexCompatible, SM: StepMode>(pub T, std::marker::PhantomData<SM>);
/*impl<T: VertexCompatible, SM: StepMode> SimpleVertex<T, SM> {

}*/

impl<T: VertexCompatible, SM: StepMode> VertexBuffer for SimpleVertex<T, SM> {
    const STRIDE: u32 = 1;
    const STEP_MODE: wgpu::VertexStepMode = SM::STEP_MODE;
    // fn desc(vertex_attrs: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout<'_> {
    //     use std::mem;
    //     wgpu::VertexBufferLayout {
    //         array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
    //         step_mode: SM::STEP_MODE,
    //         attributes: vertex_attrs,
    //     }
    // }
    fn attrs(location: u32) -> Vec<wgpu::VertexAttribute> {
        vec![wgpu::VertexAttribute {
            offset: 0,
            shader_location: location + 0,
            format: T::FORMAT,
        }]
    }
}

impl<SM: StepMode> From<TextureVertex> for SimpleVertex<[f32; 3], SM> {
    fn from(value: TextureVertex) -> Self {
        Self(value.position, std::marker::PhantomData)
    }
}
impl<SM: StepMode> From<ModelVertex> for SimpleVertex<[f32; 3], SM> {
    fn from(value: ModelVertex) -> Self {
        Self(value.position, std::marker::PhantomData)
    }
}
impl<SM: StepMode> From<ColorVertex> for SimpleVertex<[f32; 3], SM> {
    fn from(value: ColorVertex) -> Self {
        Self(value.position, std::marker::PhantomData)
    }
}

impl<T: VertexCompatible, SM: StepMode> From<T> for SimpleVertex<T, SM> {
    fn from(value: T) -> Self {
        Self(value, std::marker::PhantomData)
    }
}
/// i dont kno<w why this exists but it will be useful soon. i trhink
/*#[derive(Debug, Clone)]
pub(crate) enum VertexType {
    Color,
    Texture,
    Model,
    Simple,
}*/
pub trait StepMode: Pod + Zeroable {
    const STEP_MODE: wgpu::VertexStepMode;
}
#[repr(transparent)]
#[derive(Pod, Zeroable, Clone, Copy, Debug)]
pub struct StepVertex;
#[repr(transparent)]
#[derive(Pod, Zeroable, Clone, Copy, Debug)]
pub struct StepInstance;
impl StepMode for StepVertex {
    const STEP_MODE: wgpu::VertexStepMode = wgpu::VertexStepMode::Vertex;
}
impl StepMode for StepInstance {
    const STEP_MODE: wgpu::VertexStepMode = wgpu::VertexStepMode::Instance;
}

pub use vertex_impls::*;

#[rustfmt::skip]
mod vertex_impls {
    use bytemuck::{Pod, Zeroable};
    use std::fmt::Debug;

    pub trait VertexCompatible: Pod + Zeroable + Clone + Copy + Debug {
        const FORMAT: wgpu::VertexFormat;
    }

    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, Pod, Zeroable)]
    pub struct Unorm8(pub u8);
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, Pod, Zeroable)]
    pub struct Unorm16(pub u16);
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, Pod, Zeroable)]
    pub struct Snorm8(pub i8);
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, Pod, Zeroable)]
    pub struct Snorm16(pub i16);


    // Float formats
    impl VertexCompatible for [f32; 2] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Float32x2; }
    impl VertexCompatible for [f32; 3] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Float32x3; }
    impl VertexCompatible for [f32; 4] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Float32x4; }

    // Unsigned 8-bit integer
    impl VertexCompatible for u8 { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Uint8; }
    impl VertexCompatible for [u8; 2] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Uint8x2; }
    impl VertexCompatible for [u8; 4] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Uint8x4; }

    // Signed 8-bit integer
    impl VertexCompatible for i8 { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Sint8; }
    impl VertexCompatible for [i8; 2] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Sint8x2; }
    impl VertexCompatible for [i8; 4] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Sint8x4; }

    // Normalized unsigned 8-bit
    impl VertexCompatible for [Unorm8; 2] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Unorm8x2; }
    impl VertexCompatible for [Unorm8; 4] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Unorm8x4; }

    // Normalized signed 8-bit
    impl VertexCompatible for [Snorm8; 2] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Snorm8x2; }
    impl VertexCompatible for [Snorm8; 4] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Snorm8x4; }

    // Unsigned 16-bit integer
    impl VertexCompatible for u16 { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Uint16; }
    impl VertexCompatible for [u16; 2] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Uint16x2; }
    impl VertexCompatible for [u16; 4] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Uint16x4; }

    // Signed 16-bit integer
    impl VertexCompatible for i16 { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Sint16; }
    impl VertexCompatible for [i16; 2] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Sint16x2; }
    impl VertexCompatible for [i16; 4] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Sint16x4; }

    // Normalized unsigned 16-bit
    impl VertexCompatible for [Unorm16; 2] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Unorm16x2; }
    impl VertexCompatible for [Unorm16; 4] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Unorm16x4; }

    // Normalized signed 16-bit
    impl VertexCompatible for [Snorm16; 2] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Snorm16x2; }
    impl VertexCompatible for [Snorm16; 4] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Snorm16x4; }

    // Uint32 / Int32
    impl VertexCompatible for u32 { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Uint32; }
    impl VertexCompatible for [u32; 2] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Uint32x2; }
    impl VertexCompatible for [u32; 3] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Uint32x3; }
    impl VertexCompatible for [u32; 4] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Uint32x4; }

    impl VertexCompatible for i32 { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Sint32; }
    impl VertexCompatible for [i32; 2] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Sint32x2; }
    impl VertexCompatible for [i32; 3] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Sint32x3; }
    impl VertexCompatible for [i32; 4] { const FORMAT: wgpu::VertexFormat = wgpu::VertexFormat::Sint32x4; }
}
