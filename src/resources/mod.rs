use anyhow::{Context, anyhow};
use glam::{Quat, Vec3};
use std::io::{BufReader, Cursor};
use wgpu::ShaderModule;

// ???

mod texture;

pub use texture::*;

use crate::mesh;
use crate::mesh::vertex::ModelVertex;
use crate::prelude::Instance;

/// load to string with `file_name` appended to res path
pub async fn load_string(file_name: &str, path: &Option<String>) -> anyhow::Result<String> {
    #[cfg(not(target_arch = "wasm32"))]
    let txt = {
        let res = match path {
            Some(s) => s,
            None => {
                return Err(anyhow!(
                    "x_x :: tried to load string when the res path has not been set!"
                ));
            }
        };
        let path = std::path::Path::new(&res).join(file_name);
        std::fs::read_to_string(&path).with_context(|| path.to_str().unwrap().to_owned())?
    };

    Ok(txt)
}
/// load to binary with `file_name` appended to res path
pub async fn load_binary(file_name: &str, path: &Option<String>) -> anyhow::Result<Vec<u8>> {
    #[cfg(not(target_arch = "wasm32"))]
    let data = {
        let res = match path {
            Some(s) => s,
            None => {
                return Err(anyhow!(
                    "x_x :: tried to load string when the res path has not been set!"
                ));
            }
        };
        let path = std::path::Path::new(&res).join(file_name);
        std::fs::read(path)?
    };

    Ok(data)
}

/// load texture from res/
pub async fn load_texture(
    file_name: &str,
    context: &mut crate::Context,
) -> anyhow::Result<Texture> {
    let data = pollster::block_on(load_binary(file_name, &context.resources_path))
        .with_context(|| file_name.to_owned())?;
    texture::Texture::from_bytes(
        &context.renderer.device,
        &context.renderer.queue,
        &data,
        file_name,
    )
    // context.renderer.shader_resources.insert(resource_name, Box::new(tex));
    // let id = resource_name.to_index(&mut context.renderer.shader_resources);
    // if !context.renderer.shader_resources.is_alive(id) {
    //     context.renderer.shader_resources.insert(id, tex);
    // }
    // id
}
/// blocks
pub fn load_shader(
    shader_path: &str,
    context: &mut crate::Context,
) -> anyhow::Result<ShaderModule> {
    let vert_text = pollster::block_on(crate::resources::load_string(
        shader_path,
        &context.resources_path,
    ))?;
    let vert_descriptor = wgpu::ShaderModuleDescriptor {
        label: Some(shader_path),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&vert_text)),
    };
    Ok(context
        .renderer
        .device
        .create_shader_module(vert_descriptor))
}

mod pipeline;
pub use pipeline::*;

/// loads an .obj model using the .omtl of the same name for materials
// TODO this removes the customizability of load_shader so. fix that
///
/// .omi (ostinato material info) is info on how to construct a material from
/// .omtl (ostinto mtl) is a bundle of material infos
pub async fn load_model(
    file_name: &str,
    //device: &wgpu::Device,
    //queue: &wgpu::Queue,
    //layout: &wgpu::BindGroupLayout,
    context: &mut crate::Context,
) -> anyhow::Result<mesh::Model<ModelVertex>> {
    //let device = state.graphics().device.clone();
    //let queue = &wgpu.queue;

    let obj_text = load_string(&format!("{file_name}.obj"), &context.resources_path)
        .await
        .with_context(|| "while loading obj")?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);
    let path = &context.resources_path;
    let (models, _) =
        tobj::load_obj_buf_async(&mut obj_reader, &tobj::GPU_LOAD_OPTIONS, |p| async move {
            //println!("{p}");
            let mat_text = load_string(&p, path).await.unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        })
        .await?;

    let meshes = models
        .into_iter()
        .map(|m| {
            //println!("{m:#?}");
            let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| {
                    if m.mesh.normals.is_empty() {
                        mesh::vertex::ModelVertex {
                            position: [
                                m.mesh.positions[i * 3],
                                m.mesh.positions[i * 3 + 1],
                                m.mesh.positions[i * 3 + 2],
                            ],
                            tex_coords: [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]],
                            normal: [0.0, 0.0, 0.0],
                        }
                    } else {
                        mesh::vertex::ModelVertex {
                            position: [
                                m.mesh.positions[i * 3],
                                m.mesh.positions[i * 3 + 1],
                                m.mesh.positions[i * 3 + 2],
                            ],
                            tex_coords: [
                                m.mesh.texcoords[i * 2],
                                1.0 - m.mesh.texcoords[i * 2 + 1],
                            ],
                            normal: [
                                m.mesh.normals[i * 3],
                                m.mesh.normals[i * 3 + 1],
                                m.mesh.normals[i * 3 + 2],
                            ],
                        }
                    }
                })
                .collect::<Vec<_>>();

            /*let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", file_name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", file_name)),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });*/
            //let d = Value::Number(serde_yaml::Number::from(0));

            mesh::Mesh::new(
                vertices,
                m.mesh.indices,
                Default::default(),
                &mut context.renderer,
            )
        })
        .collect::<Vec<_>>();

    Ok(mesh::Model {
        meshes,
        transform: Instance {
            position: Vec3::ZERO,
            pivot: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        },
    })
}
pub mod blinn_phong {
    use crate::resources::load_shader;

    // const BLINN_PHONG: &'static str = include_str!("../../res/core_shaders/blinn_phong.wgsl");
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Material {
        pub ambient: [f32; 3],
        pub diffuse: [f32; 3],
        pub specular: [f32; 3],
        pub shininess: f32,
    }
    impl Material {
        pub fn to_raw(self) -> RawMaterial {
            RawMaterial {
                ambient: self.ambient,
                diffuse: self.diffuse,
                specular: self.specular,
                shininess: self.shininess,
                ..Default::default()
            }
        }
    }
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
    pub struct RawMaterial {
        ambient: [f32; 3],
        _pad0: f32,
        diffuse: [f32; 3],
        _pad1: f32,
        specular: [f32; 3],
        shininess: f32,
    }
    pub fn pipeline(
        context: &mut crate::Context,
        shader_path: &str,
        primitive: Option<wgpu::PrimitiveState>,
        depth_compare: wgpu::CompareFunction,
        buffers: &[wgpu::VertexBufferLayout<'_>],
    ) -> wgpu::RenderPipeline {
        let primitive = primitive.unwrap_or(wgpu::PrimitiveState {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        });
        let device = context.renderer.device.clone();
        let module = load_shader(shader_path, context).unwrap();
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blinn phong pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers,
            },
            primitive,
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::resources::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: context.renderer.config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::all(),
                })],
            }),
            multiview_mask: None,
            cache: None,
        })
    }
    pub fn light_binding(device: &wgpu::Device, light_buffer: &wgpu::Buffer) -> wgpu::BindGroup {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
        })
    }
}
