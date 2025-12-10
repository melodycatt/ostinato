
mod vertex;
mod model;
mod material;
pub use vertex::*;
pub use model::*;
pub use material::*;

use std::fmt::Debug;
use std::ops::Range;
use wgpu::{util::DeviceExt, Device, Queue};

use crate::Renderer;

#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<u8>,
    pub vertex_type: VertexType,
    pub indices: Vec<u32>,
    
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,

    pub num_elements: u32,

    pub material: usize
}

impl Mesh {
    /// idk what name is for
    pub fn new<T: Vertex+Debug>(
        verts: impl Into<Vec<T>>, 
        indices: impl Into<Vec<u32>>, 
        shader: usize, 
        renderer: &mut Renderer
    ) -> Self {
        let device = renderer.device.clone();
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
            vertices: bytemuck::cast_slice(&verts).to_vec(),
            vertex_type: T::TYPE,

            num_elements: indices.len() as u32,

            indices,
            vertex_buffer,
            index_buffer,

            material: shader
        }
    }
    
    pub fn update_buffers(&mut self, device: &Device, queue: &Queue) {
        //let vertex_bytes = bytemuck::cast_slice(&self.vertices);
        let vertex_data_size = self.vertices.len() as u64;

        // Create the staging buffer
        let staging_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Staging Buffer"),
            contents: &self.vertices,
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

pub fn new_cube(position: [f32; 3], scale: [f32; 3], shader_path: &str, renderer: &mut Renderer) -> Mesh {
    let verts = &[
            ModelVertex {
                position: [0.0 + position[0], 0.0 + position[1], 0.0 + position[2]],
                tex_coords: [0.0, 0.0],
                normal: [0., 0., 0.]
            },
            ModelVertex {
                position: [scale[0] + position[0], 0.0 + position[1], 0.0 + position[2]],
                tex_coords: [0.0, 0.0],
                normal: [0., 0., 0.]
            },
            ModelVertex {
                position: [0.0 + position[0], scale[1] + position[1], 0.0 + position[2]],
                tex_coords: [0.0, 0.0],
                normal: [0., 0., 0.]
            },
            ModelVertex {
                position: [scale[0] + position[0], scale[1] + position[1], 0.0 + position[2]],
                tex_coords: [0.0, 0.0],
                normal: [0., 0., 0.]
            },
            ModelVertex {
                position: [0.0 + position[0], 0.0 + position[1], scale[2] + position[2]],
                tex_coords: [0.0, 0.0],
                normal: [0., 0., 0.]
            },
            ModelVertex {
                position: [scale[0] + position[0], 0.0 + position[1], scale[2] + position[2]],
                tex_coords: [0.0, 0.0],
                normal: [0., 0., 0.]
            },
            ModelVertex {
                position: [0.0 + position[0], scale[1] + position[1], scale[2] + position[2]],
                tex_coords: [0.0, 0.0],
                normal: [0., 0., 0.]
            },
            ModelVertex {
                position: [scale[0] + position[0], scale[1] + position[1], scale[2] + position[2]],
                tex_coords: [0.0, 0.0],
                normal: [0., 0., 0.]
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
    Mesh::new(verts, indices, renderer.materials.index_of(shader_path), renderer)
}

pub trait DrawModel<'a> {
    fn draw_mesh(&mut self, mesh: &'a Mesh);
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        instances: Range<u32>,
    );
}
impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(&mut self, mesh: &'b Mesh) {
        self.draw_mesh_instanced(mesh, 0..1);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
    ){
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }
}