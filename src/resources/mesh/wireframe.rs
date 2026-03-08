use std::{fmt::Debug, ops::Range};

use anyhow::anyhow;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BufferUsages, ShaderStages, util::DeviceExt,
};

use crate::{
    prelude::{Instance, Mesh, Renderable, Renderer},
    resources::{SimpleVertex, StepVertex, VertexBuffer},
};

/// LOLOLOLOLOL i spent ages trying to make this work (poor docs for immediates) and apparently i
/// can just do PolygonMode::Line
/// a mesh that stores its vertices and indices in storage buffers instead of special buffers
/// for wireframes but use it how you want
/// reserve bind group 1 for the storage buffers
///
/// TODO: so this isnt completely useless
/// currently this can only use SimpleVertex because it was convenient for my wireframe
/// same story for the color field
/// if i actually want to make this useful for anything else ill get change all of that but for now im
/// fine so :)
#[derive(Debug, Clone)]
pub struct StorageMesh {
    // todo make this Vec<T> ? i think
    // unless i cant
    pub vertices: Vec<SimpleVertex<[f32; 3], StepVertex>>,
    // todo make this Vec<u32>
    pub indices: Vec<u8>,

    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub transform: Instance,

    /// indices.len()
    pub num_elements: u32,

    /// resource index of material
    pub material: usize,
    bind_group: BindGroup,
    pub color: [f32; 4],
}

impl StorageMesh {
    pub fn new(
        vertices: impl Into<Vec<SimpleVertex<[f32; 3], StepVertex>>>,
        indices: impl Into<Vec<u32>>,
        shader: usize,
        renderer: &mut Renderer,
    ) -> Self {
        let device = renderer.device.clone();
        let vbinding = vertices.into();
        let vertices = bytemuck::cast_slice(&vbinding);
        let ibinding = indices.into();
        let num_elements = ibinding.len() as u32 * 2;
        let indices = bytemuck::cast_slice(&ibinding);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: vertices,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: indices,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = Self::bind_group(&vertex_buffer, &index_buffer, &device);

        Self {
            vertices: vbinding,
            num_elements,

            indices: indices.to_vec(),
            vertex_buffer,
            index_buffer,

            transform: Instance::default(),
            material: shader,

            bind_group,
            color: [1., 0., 0., 1.],
        }
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("storagmesh storage bind group"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }
    pub fn bind_group(
        vbuf: &wgpu::Buffer,
        ibuf: &wgpu::Buffer,
        device: &wgpu::Device,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("storagemesh verts and indices bind group"),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: vbuf.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: ibuf.as_entire_binding(),
                },
            ],
            layout: &Self::bind_group_layout(device),
        })
    }

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
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
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
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
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

    /// Convert a Mesh into a StorageMesh by consuming it
    pub fn from_mesh<V: Into<SimpleVertex<[f32; 3], StepVertex>> + VertexBuffer + Debug>(
        mesh: Mesh<V>,
        device: &wgpu::Device,
    ) -> anyhow::Result<StorageMesh> {
        let vertices: Vec<SimpleVertex<[f32; 3], StepVertex>> =
            mesh.vertices.into_iter().map(|x| x.into()).collect();
        // Create storage buffer for vertices
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("StorageMesh Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        // Create storage buffer for indices
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("StorageMesh Index Buffer"),
            contents: &mesh.indices,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        // Create bind group (reserve group 1)
        let bind_group = Self::bind_group(&vertex_buffer, &index_buffer, device);

        Ok(StorageMesh {
            vertices,              // moved
            indices: mesh.indices, // moved
            vertex_buffer,
            index_buffer,
            transform: mesh.transform, // moved
            num_elements: mesh.num_elements * 2,
            material: mesh.material,
            bind_group,
            color: [1., 0., 0., 1.],
        })
    }
}

impl Renderable for StorageMesh {
    fn draw(
        &self,
        pass: &mut wgpu::RenderPass,
        manual_bindings: &[BindGroup],
        renderer: &mut Renderer,
    ) -> anyhow::Result<()> {
        self.draw_instances(pass, manual_bindings, 0..1, renderer)
    }
    fn draw_instances(
        &self,
        pass: &mut wgpu::RenderPass,
        manual_bindings: &[BindGroup],
        instances: Range<u32>,
        renderer: &mut Renderer,
    ) -> anyhow::Result<()> {
        let m = renderer
            .materials
            .get(self.material)
            .ok_or(anyhow!("x_x :: todo write this panic message"))?;
        pass.set_pipeline(&m.render_pipeline);

        let mut manual_i = 0;
        for i in 0..m.bind_groups.len() {
            if i == 1 {
                pass.set_bind_group(1, &self.bind_group, &[]);
                continue;
            }
            let b = &m.bind_groups[i];
            if b.is_some() {
                pass.set_bind_group(i as u32, b, &[]);
            } else {
                pass.set_bind_group(i as u32, Some(&manual_bindings[manual_i]), &[]);
                manual_i += 1;
            }
        }
        pass.set_bind_group(1, &self.bind_group, &[]);
        pass.set_immediates(112, bytemuck::cast_slice(&[self.color]));
        pass.set_immediates(0, bytemuck::cast_slice(&[self.transform.to_raw()]));
        //println!("drawing 0..{}", self.num_elements);
        pass.draw(0..self.num_elements, instances);
        Ok(())
    }
}
