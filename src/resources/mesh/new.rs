use std::ops::Range;

use anyhow::anyhow;
use wgpu::util::DeviceExt;

use crate::{
    Context,
    prelude::{Instance, Renderable, Renderer},
    resources::{
        ModelVertex, VertexBuffer,
        pipeline::{BlinnPhong, GeometryPass, LazyMaterial, LazyPass, MaterialType},
    },
};

pub struct Mesh<Material, V: VertexBuffer> {
    pub material: Material,
    pub vertices: Vec<V>,
    // todo make this Vec<u32>
    pub indices: Vec<u8>,

    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub transform: Instance,

    /// indices.len()
    pub num_elements: u32,
}

impl<M, V: VertexBuffer> Mesh<M, V> {
    /// idk what name is for
    pub fn new(
        vertices: impl Into<Vec<V>>,
        indices: impl Into<Vec<u32>>,
        material: M,
        renderer: &mut Renderer,
    ) -> Self {
        let device = renderer.device.clone();
        let vbinding = vertices.into();
        let vertices = bytemuck::cast_slice(&vbinding);
        let ibinding = indices.into();
        let num_elements = ibinding.len() as u32;
        let indices = bytemuck::cast_slice(&ibinding);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: vertices,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: indices,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            vertices: vbinding,
            num_elements,

            indices: indices.to_vec(),
            vertex_buffer,
            index_buffer,

            transform: Instance::default(),
            material,
        }
    }

    // todo add set_indices and set_vertices which convert to Vec<u8>

    /// updates the data stored in the vertex and index buffers
    /// the instance buffer is updated separately with `InstanceInfo::update_buffer()`
    /// if the data for a buffer is larger than the current buffer size it gets reallocated
    /// currently allocates a staging buffer for the data copied to each buffer
    /// not sure if theres a faster way to do it
    // todo if you know how too make this faster pls do
    pub fn update_buffers(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Buffer Copy Encoder"),
        });

        {
            let vertex_data_size = self.vertices.len() as u64;

            // Create the staging buffer
            let staging_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Staging Buffer"),
                contents: bytemuck::cast_slice(&self.vertices),
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

            encoder.copy_buffer_to_buffer(
                &staging_buffer,
                0,
                &self.vertex_buffer,
                0,
                vertex_data_size,
            );
        }
        {
            let index_data_size = self.indices.len() as u64;

            // Create the staging buffer
            let staging_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Staging Buffer"),
                contents: &self.indices,
                usage: wgpu::BufferUsages::COPY_SRC,
            });

            // Reuse or reallocate the vertex buffer as needed
            if self.index_buffer.size() < index_data_size {
                self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Vertex Buffer"),
                    size: index_data_size,
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }

            // Copy

            encoder.copy_buffer_to_buffer(
                &staging_buffer,
                0,
                &self.index_buffer,
                0,
                index_data_size,
            );
        }

        queue.submit(Some(encoder.finish()));
    }

    pub fn with_transform(mut self, transform: Instance) -> Self {
        self.transform = transform;
        self
    }

    // pub fn set_vertices(mut self, vertices: impl Into<Vec<T>>)
}

impl Renderable for Mesh<BlinnPhong, ModelVertex> {
    fn draw_instances(
        &self,
        pass: &mut wgpu::RenderPass,
        instances: Range<u32>,
        context: &mut Context,
    ) -> anyhow::Result<()> {
        let m = BlinnPhong::get(context);

        pass.set_pipeline(&m.pipeline);
        pass.set_bind_group(1, Some(&m.bind_group), &[]);
        pass.set_immediates(0, bytemuck::bytes_of(&self.transform.to_raw()));
        self.material.prerender(pass);
        let p = GeometryPass::get(context);
        pass.set_bind_group(0, Some(&p.bind_group), &[]);

        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.num_elements, 0, instances);
        Ok(())
    }
}
