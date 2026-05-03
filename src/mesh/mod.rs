pub mod vertex;
use glam::{Quat, Vec3};
use vertex::*;
// mod vertex;
mod model;
mod wireframe;
pub use model::*;
pub use wireframe::*;

use std::{fmt::Debug, marker::PhantomData};
use wgpu::{Device, Queue, util::DeviceExt};

use crate::{
    Renderer,
    prelude::Renderable,
    renderer::{Instance, InstanceRaw},
    resources::blinn_phong::Material,
};

/// a mesh for renderingi
#[derive(Debug, Clone)]
pub struct Mesh<V: VertexBuffer> {
    _phantom: PhantomData<V>,
    // todo make this Vec<T> ? i think
    // unless i cant
    pub vertices: Vec<V>,
    // todo make this Vec<u32>
    pub indices: Vec<u8>,

    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub transform: Instance,

    /// indices.len()
    pub num_elements: u32,
    pub material: Material,
}

#[derive(Debug, Clone)]
pub struct InstanceInfo {
    pub buffer: wgpu::Buffer,
    pub instances: Vec<Instance>,
}
impl InstanceInfo {
    pub fn new(instances: Vec<Instance>, renderer: &mut Renderer) -> Self {
        let raw_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();

        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh Instance Buffer"),
                contents: bytemuck::cast_slice(&raw_data),
                usage: wgpu::BufferUsages::VERTEX,
            });

        Self { instances, buffer }
    }
    /// converts instances to raw byte data and writes them to the instance buffer
    /// allocates staging buffer
    pub fn update_buffer(&mut self, device: &Device, queue: &Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Instance Buffer Copy Encoder"),
        });
        let instance_raws = self
            .instances
            .iter()
            .map(Instance::to_raw)
            .collect::<Vec<_>>();
        let raw_data = bytemuck::cast_slice(&instance_raws);
        let raw_data_size = raw_data.len() as u64;

        // Create the staging buffer
        let staging_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Staging Buffer"),
            contents: raw_data,
            usage: wgpu::BufferUsages::COPY_SRC,
        });

        // Reuse or reallocate the vertex buffer as needed
        if self.buffer.size() < raw_data_size {
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Instance Buffer"),
                size: raw_data_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        // Copy

        //dbg!(raw_data_size);
        encoder.copy_buffer_to_buffer(&staging_buffer, 0, &self.buffer, 0, raw_data_size);

        queue.submit(Some(encoder.finish()));
    }
}

impl<V: VertexBuffer> Mesh<V> {
    /// idk what name is for
    pub fn new(
        vertices: impl Into<Vec<V>>,
        indices: impl Into<Vec<u32>>,
        material: Material,
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
            material,
            transform: Instance {
                position: Vec3::ZERO,
                pivot: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
            _phantom: PhantomData,
        }
    }

    // todo add set_indices and set_vertices which convert to Vec<u8>

    /// updates the data stored in the vertex and index buffers
    /// the instance buffer is updated separately with `InstanceInfo::update_buffer()`
    /// if the data for a buffer is larger than the current buffer size it gets reallocated
    /// currently allocates a staging buffer for the data copied to each buffer
    /// not sure if theres a faster way to do it
    // todo if you know how too make this faster pls do
    pub fn update_buffers(&mut self, device: &Device, queue: &Queue) {
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

pub fn new_cube(
    instance: Instance,
    material: Material,
    renderer: &mut Renderer,
) -> Mesh<ModelVertex> {
    const UNIT_CUBE_VERTICES: [ModelVertex; 24] = [
        // +Z
        ModelVertex {
            position: [0.0, 0.0, 1.0],
            tex_coords: [0.0, 0.0],
            normal: [0.0, 0.0, 1.0],
        },
        ModelVertex {
            position: [1.0, 0.0, 1.0],
            tex_coords: [1.0, 0.0],
            normal: [0.0, 0.0, 1.0],
        },
        ModelVertex {
            position: [1.0, 1.0, 1.0],
            tex_coords: [1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
        },
        ModelVertex {
            position: [0.0, 1.0, 1.0],
            tex_coords: [0.0, 1.0],
            normal: [0.0, 0.0, 1.0],
        },
        // -Z
        ModelVertex {
            position: [1.0, 0.0, 0.0],
            tex_coords: [0.0, 0.0],
            normal: [0.0, 0.0, -1.0],
        },
        ModelVertex {
            position: [0.0, 0.0, 0.0],
            tex_coords: [1.0, 0.0],
            normal: [0.0, 0.0, -1.0],
        },
        ModelVertex {
            position: [0.0, 1.0, 0.0],
            tex_coords: [1.0, 1.0],
            normal: [0.0, 0.0, -1.0],
        },
        ModelVertex {
            position: [1.0, 1.0, 0.0],
            tex_coords: [0.0, 1.0],
            normal: [0.0, 0.0, -1.0],
        },
        // +X
        ModelVertex {
            position: [1.0, 0.0, 1.0],
            tex_coords: [0.0, 0.0],
            normal: [1.0, 0.0, 0.0],
        },
        ModelVertex {
            position: [1.0, 0.0, 0.0],
            tex_coords: [1.0, 0.0],
            normal: [1.0, 0.0, 0.0],
        },
        ModelVertex {
            position: [1.0, 1.0, 0.0],
            tex_coords: [1.0, 1.0],
            normal: [1.0, 0.0, 0.0],
        },
        ModelVertex {
            position: [1.0, 1.0, 1.0],
            tex_coords: [0.0, 1.0],
            normal: [1.0, 0.0, 0.0],
        },
        // -X
        ModelVertex {
            position: [0.0, 0.0, 0.0],
            tex_coords: [0.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
        },
        ModelVertex {
            position: [0.0, 0.0, 1.0],
            tex_coords: [1.0, 0.0],
            normal: [-1.0, 0.0, 0.0],
        },
        ModelVertex {
            position: [0.0, 1.0, 1.0],
            tex_coords: [1.0, 1.0],
            normal: [-1.0, 0.0, 0.0],
        },
        ModelVertex {
            position: [0.0, 1.0, 0.0],
            tex_coords: [0.0, 1.0],
            normal: [-1.0, 0.0, 0.0],
        },
        // +Y
        ModelVertex {
            position: [0.0, 1.0, 0.0],
            tex_coords: [0.0, 0.0],
            normal: [0.0, 1.0, 0.0],
        },
        ModelVertex {
            position: [0.0, 1.0, 1.0],
            tex_coords: [1.0, 0.0],
            normal: [0.0, 1.0, 0.0],
        },
        ModelVertex {
            position: [1.0, 1.0, 1.0],
            tex_coords: [1.0, 1.0],
            normal: [0.0, 1.0, 0.0],
        },
        ModelVertex {
            position: [1.0, 1.0, 0.0],
            tex_coords: [0.0, 1.0],
            normal: [0.0, 1.0, 0.0],
        },
        // -Y
        ModelVertex {
            position: [0.0, 0.0, 0.0],
            tex_coords: [0.0, 0.0],
            normal: [0.0, -1.0, 0.0],
        },
        ModelVertex {
            position: [1.0, 0.0, 0.0],
            tex_coords: [1.0, 0.0],
            normal: [0.0, -1.0, 0.0],
        },
        ModelVertex {
            position: [1.0, 0.0, 1.0],
            tex_coords: [1.0, 1.0],
            normal: [0.0, -1.0, 0.0],
        },
        ModelVertex {
            position: [0.0, 0.0, 1.0],
            tex_coords: [0.0, 1.0],
            normal: [0.0, -1.0, 0.0],
        },
    ];

    const UNIT_CUBE_INDICES: [u32; 36] = [
        0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15, 16, 17,
        18, 16, 18, 19, 20, 21, 22, 20, 22, 23,
    ];
    // const MATERIAL: Material = Material {
    //     ambient: [0.1, 0.1, 0.1],
    //     diffuse: [1., 1., 1.],
    //     specular: [1., 1., 1.],
    //     shininess: 64.,
    // };
    Mesh::new(UNIT_CUBE_VERTICES, UNIT_CUBE_INDICES, material, renderer).with_transform(instance)
}

#[derive(Debug, Clone)]
pub struct InstancedMesh<V: VertexBuffer> {
    pub mesh: Mesh<V>,
    pub instances: InstanceInfo,
}
impl<V: VertexBuffer> InstancedMesh<V> {
    pub fn new(mesh: Mesh<V>, instances: Vec<Instance>, renderer: &mut Renderer) -> Self {
        Self {
            mesh,
            instances: InstanceInfo::new(instances, renderer),
        }
    }
}

impl<V: VertexBuffer> Renderable for InstancedMesh<V> {
    fn draw(&self, pass: &mut wgpu::RenderPass, renderer: &mut Renderer) {
        self.draw_instances(pass, 0..self.instances.instances.len() as u32, renderer);
    }
    fn draw_instances(
        &self,
        pass: &mut wgpu::RenderPass,
        instances: std::ops::Range<u32>,
        _: &mut Renderer,
    ) {
        pass.set_index_buffer(self.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.set_vertex_buffer(0, self.mesh.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instances.buffer.slice(..));
        pass.set_immediates(0, bytemuck::cast_slice(&[self.mesh.transform.to_raw()]));
        pass.set_immediates(
            std::mem::size_of::<InstanceRaw>() as u32,
            bytemuck::bytes_of(&self.mesh.material.to_raw()),
        );

        pass.draw_indexed(0..self.mesh.num_elements, 0, instances);
    }
}
impl<V: VertexBuffer> Renderable for Mesh<V> {
    fn draw_instances(
        &self,
        pass: &mut wgpu::RenderPass,
        instances: std::ops::Range<u32>,
        _: &mut Renderer,
    ) {
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_immediates(0, bytemuck::cast_slice(&[self.transform.to_raw()]));
        pass.set_immediates(
            std::mem::size_of::<InstanceRaw>() as u32,
            bytemuck::bytes_of(&self.material.to_raw()),
        );
        pass.draw_indexed(0..self.num_elements, 0, instances);
    }
}
