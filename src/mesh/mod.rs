
mod vertex;
use vectors::Vector2;
pub use vertex::*;
use wgpu::{util::DeviceExt, Buffer, Device, Queue};
pub struct Mesh {
    pub verts: Vec<Vertex>,
    pub indices: Vec<u16>,
    
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
}

impl Mesh {
    pub fn new(verts: impl Into<Vec<Vertex>>, indices: impl Into<Vec<u16>>, device: &Device) -> Self {
        let verts = verts.into();
        let indices = indices.into();
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(verts.iter().map(|v| v.wgpu_map()).collect::<Vec<_>>().as_slice()),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(indices.as_slice()),
                usage: wgpu::BufferUsages::INDEX,
            }
        );
        Self {
            verts: verts,   
            indices: indices,
            vertex_buffer,
            index_buffer
        }
    }

    pub fn update_buffers(&mut self, device: &Device, queue: &Queue) {
        let vertex_data: Vec<_> = self.verts.iter().map(|v| v.wgpu_map()).collect();
        let vertex_bytes = bytemuck::cast_slice(&vertex_data);
        let vertex_data_size = vertex_bytes.len() as u64;

        // Create the staging buffer
        let staging_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Staging Buffer"),
            contents: vertex_bytes,
            usage: wgpu::BufferUsages::COPY_SRC,
        });

        // Reuse or reallocate the vertex buffer as needed
        if self.vertex_buffer.size() < vertex_data_size {
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Vertex Buffer"),
                size: vertex_data_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        // Copy
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Vertex Buffer Copy Encoder"),
        });

        encoder.copy_buffer_to_buffer(&staging_buffer, 0, &self.vertex_buffer, 0, vertex_data_size);

        queue.submit(Some(encoder.finish()));
    }

}