
mod vertex;
use vectors::Vector;
pub use vertex::*;
use wgpu::{util::DeviceExt, Device};
pub struct Mesh {
    pub verts: Vec<Vertex>,
    pub vertex_buffer: wgpu::Buffer
}

impl Mesh {
    pub fn new(verts: &[Vertex], device: &Device) -> Self {
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(verts.iter().map(|v| v.wgpu_map()).collect::<Vec<_>>().as_slice()),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        Self {
            verts: verts.to_vec(),   
            vertex_buffer
        }
    }
}