
mod vertex;
use std::{fs, path::Path};

pub use vertex::*;
use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout, Device, Queue};

use crate::{shader::{self, Shader}, State};

pub struct Mesh {
    pub verts: Vec<Vertex>,
    pub indices: Vec<u16>,
    
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,

    pub shader: Shader
}

impl Mesh {
    pub fn new(verts: impl Into<Vec<Vertex>>, indices: impl Into<Vec<u16>>, shader: Shader, state: &State) -> Self {
        let device = state.wgpu().device.clone();
        let verts = verts.into();
        let indices = indices.into();
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        Self {
            verts,   
            indices,
            vertex_buffer,
            index_buffer,

            shader
        }
    }
    
    pub fn construct<P: AsRef<Path>>(verts: impl Into<Vec<Vertex>>, indices: impl Into<Vec<u16>>, bind_groups: &[BindGroup], layouts: &[&BindGroupLayout], shader_path: P, state: &mut State) -> Self {
        let device = state.wgpu().device.clone();

        let f = fs::read_to_string(shader_path).expect("shader non existent in creating cube");
        let descriptor = wgpu::ShaderModuleDescriptor {
            label: Some("mesh shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&f)),
        };
    // include_wgsl!()
        let shader = device.create_shader_module(descriptor);

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &state.camera().bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: state.camera().buffer.as_entire_binding(),
                }   
            ],    
            label: Some("mesh camera bind group"),
        });

        Mesh::new(
            verts, indices,
            shader::Shader::new(shader, [&[camera_bind_group], bind_groups].concat(), state.render_pipeline_layout(layouts) /*&[Arc::new(camera_bind_group_layout), Arc::new(time_bind_group_layout)]*/, &[Vertex::desc()], &[Some(shader::Shader::screen_target(state.wgpu().config.format))], &device),
            &state
        )
    }

    pub fn update_buffers(&mut self, device: &Device, queue: &Queue) {
        let vertex_bytes = bytemuck::cast_slice(&self.verts);
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

/*impl RenderObject for Mesh {
    fn render(&mut self, pass: &mut wgpu::RenderPass) {
        pass.set_pipeline(&self.shader.render_pipeline);
        for i in 0..self.shader.bind_groups.len() {
            pass.set_bind_group(i as u32, &self.shader.bind_groups[i], &[]);
        }
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
    }
}*/

pub fn new_cube<P: AsRef<Path>>(position: [f32; 3], bind_groups: &[BindGroup], layouts: &[&BindGroupLayout], shader_path: P, state: &mut State) -> Mesh {
    let verts = &[
            Vertex {
                position: [0.0 + position[0], 0.0 + position[1], 0.0 + position[2]],
                color: [1.0, 0.0, 0.0, 1.0],
            },
            Vertex {
                position: [1.0 + position[0], 0.0 + position[1], 0.0 + position[2]],
                color: [1.0, 0.0, 0.0, 0.8],
            },
            Vertex {
                position: [0.0 + position[0], 1.0 + position[1], 0.0 + position[2]],
                color: [1.0, 0.0, 0.0, 0.8],
            },
            Vertex {
                position: [1.0 + position[0], 1.0 + position[1], 0.0 + position[2]],
                color: [1.0, 0.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.0 + position[0], 0.0 + position[1], -1.0 + position[2]],
                color: [0.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                position: [1.0 + position[0], 0.0 + position[1], -1.0 + position[2]],
                color: [0.0, 1.0, 0.0, 0.8],
            },
            Vertex {
                position: [0.0 + position[0], 1.0 + position[1], -1.0 + position[2]],
                color: [0.0, 1.0, 0.0, 0.8],
            },
            Vertex {
                position: [1.0 + position[0], 1.0 + position[1], -1.0 + position[2]],
                color: [0.0, 1.0, 0.0, 1.0],
            },
        ];
    let indices = &[
            0, 1, 2,
            1, 3, 2,   
            0, 2, 4,   
            4, 2, 6,   
            1, 5, 3,   
            5, 7, 3,   
            0, 4, 1,   
            4, 5, 1,   
            2, 3, 6,   
            6, 3, 7,   
            4, 6, 5,   
            6, 7, 5
        ];
    Mesh::construct(verts, indices, bind_groups, layouts, shader_path, state)
}