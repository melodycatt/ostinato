use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, Device};

use crate::{
    Context,
    mesh::{
        InstancedMesh, Mesh,
        vertex::{ModelVertex, VertexBuffer},
    },
    prelude::{Instance, Renderable},
    renderer::InstanceRaw,
    resources::{Texture, load_texture},
};

// todo ?
// model.rs
pub struct Model<V: VertexBuffer> {
    pub meshes: Vec<Mesh<V>>,
    pub transform: Instance,
}

impl<V: VertexBuffer> Renderable for Model<V> {
    fn draw_instances(
        &self,
        pass: &mut wgpu::RenderPass,
        instances: std::ops::Range<u32>,
        _: &mut crate::prelude::Renderer,
    ) {
        for i in 0..self.meshes.len() {
            pass.set_index_buffer(
                self.meshes[i].index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.set_vertex_buffer(0, self.meshes[i].vertex_buffer.slice(..));
            pass.set_immediates(
                0,
                bytemuck::cast_slice(&[self.transform.apply(&self.meshes[i].transform).to_raw()]),
            );
            pass.set_immediates(
                std::mem::size_of::<InstanceRaw>() as u32,
                bytemuck::bytes_of(&self.meshes[i].material.to_raw()),
            );

            pass.draw_indexed(0..self.meshes[i].num_elements, 0, instances.clone());
        }
    }
}

pub struct ObjModel {
    pub meshes: Vec<ObjMesh>,
    pub transform: Instance,
}
impl ObjModel {
    pub async fn from_model(
        model: Model<ModelVertex>,
        textures: &[[&str; 2]],
        context: &mut Context,
    ) -> anyhow::Result<Self> {
        if model.meshes.len() != textures.len() {
            panic!("a")
        }
        let bgl = ObjMesh::bind_group_layout(&context.renderer.device);

        let mut old_meshes = model.meshes.into_iter();
        let mut meshes = Vec::with_capacity(textures.len());
        for i in 0..textures.len() {
            let texture = load_texture(textures[i][0], context).await?;
            let roughness = load_texture(textures[i][1], context).await?;

            let bg = ObjMesh::bind_group(&bgl, [&texture, &roughness], &context.renderer.device);
            meshes.push(ObjMesh {
                mesh: old_meshes.next().unwrap(),
                texture,
                roughness,
                bind_group: bg,
            })
        }

        Ok(Self {
            meshes,
            transform: model.transform,
        })
    }
}

impl Renderable for ObjModel {
    fn draw_instances(
        &self,
        pass: &mut wgpu::RenderPass,
        instances: std::ops::Range<u32>,
        _: &mut crate::prelude::Renderer,
    ) {
        for i in 0..self.meshes.len() {
            pass.set_index_buffer(
                self.meshes[i].mesh.index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.set_vertex_buffer(0, self.meshes[i].mesh.vertex_buffer.slice(..));
            pass.set_bind_group(2, Some(&self.meshes[i].bind_group), &[]);
            pass.set_immediates(
                0,
                bytemuck::cast_slice(&[self
                    .transform
                    .apply(&self.meshes[i].mesh.transform)
                    .to_raw()]),
            );
            pass.set_immediates(
                std::mem::size_of::<InstanceRaw>() as u32,
                bytemuck::bytes_of(&self.meshes[i].mesh.material.to_raw()),
            );

            pass.draw_indexed(0..self.meshes[i].mesh.num_elements, 0, instances.clone());
        }
    }
}

pub struct ObjMesh {
    pub mesh: Mesh<ModelVertex>,
    pub texture: Texture,
    pub roughness: Texture,
    pub bind_group: BindGroup,
}
impl ObjMesh {
    fn bind_group_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }
    fn bind_group(bgl: &BindGroupLayout, textures: [&Texture; 2], device: &Device) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&textures[0].view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&textures[0].sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&textures[1].view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&textures[1].sampler),
                },
            ],
            label: None,
            layout: bgl,
        })
    }
}
