use anyhow::{Context, anyhow};
use glam::{Quat, Vec3};
use serde_yaml::Value;
use std::{
    io::{BufReader, Cursor},
    num::NonZero,
};
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, FragmentState, PrimitiveState, RenderPipelineDescriptor, VertexBufferLayout,
    VertexState,
};

// ???

mod material;
mod mesh;
mod texture;

mod collection;
pub use collection::*;

pub use material::*;
pub use mesh::*;
pub use texture::*;

use crate::prelude::Instance;

// TODO remove wasm
#[cfg(target_arch = "wasm32")]
fn format_url(file_name: &str) -> reqwest::Url {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let mut origin = location.origin().unwrap();
    if !origin.ends_with("learn-wgpu") {
        origin = format!("{}/learn-wgpu", origin);
    }
    let base = reqwest::Url::parse(&format!("{}/", origin,)).unwrap();
    base.join(file_name).unwrap()
}

/// load to string with `file_name` appended to res path
pub async fn load_string(file_name: &str, path: &Option<String>) -> anyhow::Result<String> {
    #[cfg(target_arch = "wasm32")]
    let txt = {
        let url = format_url(file_name);
        reqwest::get(url).await?.text().await?
    };
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
    #[cfg(target_arch = "wasm32")]
    let data = {
        let url = format_url(file_name);
        reqwest::get(url).await?.bytes().await?.to_vec()
    };
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
    resource_name: Option<&str>,
) -> usize {
    let resource_name = match resource_name {
        Some(name) => name,
        None => file_name,
    };
    let data = pollster::block_on(load_binary(file_name, &context.resources_path))
        .with_context(|| file_name.to_owned())
        .unwrap();
    let tex = texture::Texture::from_bytes(
        &context.renderer.device,
        &context.renderer.queue,
        &data,
        file_name,
    )
    .unwrap();
    // context.renderer.shader_resources.insert(resource_name, Box::new(tex));
    let id = resource_name.to_index(&mut context.renderer.shader_resources);
    if !context.renderer.shader_resources.is_alive(id) {
        context.renderer.shader_resources.insert(id, tex);
    }
    id
}

/// i hope this isnt public
macro_rules! parse_yaml {
    ($val:expr, $as:ident, $name:expr) => {
        $val.ok_or(anyhow!(
            "x_x :: invalid OMI yaml! missing `{}` in bind group",
            $name
        ))?
        .$as()
        .ok_or(anyhow!(
            "x_x :: invalid OMI yaml! field `{}` is invalid",
            $name
        ))?
    };
}
macro_rules! unwrap_yaml {
    ($val:expr, $name:expr) => {
        $val.ok_or(anyhow!(
            "x_x :: invalid OMI yaml! missing `{}` in bind group",
            $name
        ))?
    };
}

/// load a material from an .omi file
///
/// if `resource_name` is omitted `file_name` is used to store the shader in the context.renderer;
/// you should specify a resource name if you load the same shader twice with different options
///
/// `primitive_state` is for advanced render pipeline config such as:
/// - face culling
/// - polygon mode (e.g. wireframe)
/// - index format
///
/// see wgpu::PrimitiveState
pub async fn load_material(
    file_name: &str,
    context: &mut crate::Context,
    resource_name: Option<&str>,
    primitive_state: Option<PrimitiveState>,
) -> anyhow::Result<usize> {
    let resource_name = match resource_name {
        Some(name) => name,
        None => file_name,
    };
    let primitive_state = match primitive_state {
        Some(prim) => prim,
        None => PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
    };

    let id = resource_name.to_index(&mut context.renderer.materials);
    if !context.renderer.materials.is_alive(id) {
        let m = load_omi(file_name, context, primitive_state)
            .await
            .context(anyhow!("on shader: {}", resource_name))?;
        context.renderer.materials.insert(resource_name, m);
    }

    Ok(id)
}

/// such a pain to write AND badly done. the ultimate combination
/// idk why this is public but use it at your own discretion!
pub async fn load_omi(
    file_name: &str,
    context: &mut crate::Context,
    primitive_state: PrimitiveState,
) -> anyhow::Result<Material> {
    // TODO: maybe use serde_yaml as intended. rather than manually parsing. idk

    let device = context.renderer.device.clone();
    let omi_text = load_string(&format!("{file_name}.omi"), &context.resources_path).await?;
    let root: Value = serde_yaml::from_str(&omi_text)?;

    let bind_groups_yaml = match root.get("bind_groups") {
        Some(value) => value.as_sequence().ok_or(anyhow!(
            "x_x :: invalid OMI yaml! field `bind_groups` is invalid"
        ))?,
        None => &Vec::new(),
    };

    let mut bind_group_layouts = Vec::with_capacity(bind_groups_yaml.len());
    let mut bind_groups = Vec::with_capacity(bind_groups_yaml.len());
    if let Some(gymls) = root.get("shared_bind_groups") {
        let globals_yaml = gymls
            .as_sequence()
            .ok_or(anyhow!("x_x :: shared_bind_groups field is invalid"))?;
        let globals = globals_yaml
            .iter()
            .map(|x| {
                let r = x
                    .as_str()
                    .ok_or(anyhow!("x_x :: invalid shared_bind_groups item!"))
                    .map(|x| {
                        context.renderer.shared_bindings.get(x).ok_or(anyhow!(
                            "x_x :: shared bind group is nonexistent or whatever i dont careeeeeee"
                        ))
                    });
                match r {
                    Ok(v) => v,
                    Err(e) => Err(e),
                }
            })
            .collect::<anyhow::Result<Vec<_>, _>>()?;
        let (global_layouts, global_entries): (Vec<_>, Vec<_>) = globals
            .iter()
            .enumerate()
            .map(|(i, (resource, generator))| {
                (
                    generator(i as u32),
                    BindGroupEntry {
                        binding: i as u32,
                        resource: resource.binding(),
                    },
                )
            })
            .unzip();
        let globals_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("shared bindings bind group"),
                entries: &global_layouts,
            });
        let globals_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("shared bindings bind group"),
            layout: &globals_bind_group_layout,
            entries: &global_entries,
        });
        // dbg!(&global_layouts);

        bind_groups.push(Some(globals_bind_group));
        bind_group_layouts.push(globals_bind_group_layout);
    }

    for group in bind_groups_yaml.iter() {
        let label = parse_yaml!(group.get("label"), as_str, "label");
        let manual = parse_yaml!(group.get("manual"), as_bool, "manual");
        let entries_yaml = parse_yaml!(group.get("entries"), as_sequence, "entries");
        let mut entries = Vec::with_capacity(entries_yaml.len());
        let mut buffer_labels = Vec::with_capacity(if !manual { entries_yaml.len() } else { 0 });
        for (e, entry) in entries_yaml.iter().enumerate() {
            let vis = parse_yaml!(entry.get("visibility"), as_sequence, "visibility")
                .iter()
                .map(|value| match value.as_str() {
                    Some("FRAGMENT") => Ok(wgpu::ShaderStages::FRAGMENT),
                    Some("COMPUTE") => Ok(wgpu::ShaderStages::COMPUTE),
                    Some("MESH") => Ok(wgpu::ShaderStages::MESH),
                    Some("VERTEX") => Ok(wgpu::ShaderStages::VERTEX),
                    Some("NONE") => Ok(wgpu::ShaderStages::NONE),
                    Some("TASK") => Ok(wgpu::ShaderStages::TASK),
                    Some("VERTEX_FRAGMENT") => Ok(wgpu::ShaderStages::VERTEX_FRAGMENT),
                    Some(v) => Err(anyhow!(
                        "x_x :: invalid OMI yaml! invalid `visibility` value: {v}"
                    )),
                    None => Err(anyhow!(
                        "x_x :: invalid OMI yaml! `visibility` must be a string"
                    )),
                })
                .try_fold(wgpu::ShaderStages::NONE, |acc, x| x.map(|s| acc | s))?;
            let group_type = binding_type(entry, context).await?;
            let count_parse = unwrap_yaml!(entry.get("count"), "count").as_u64();
            let count = match count_parse {
                Some(x) => Some(
                    NonZero::new(x as u32)
                        .ok_or(anyhow!("x_x :: invalid OMI yaml! field `count` is zero"))?,
                ),
                None => None,
            };
            entries.push(BindGroupLayoutEntry {
                binding: e as u32,
                visibility: vis,
                ty: group_type,
                count,
            });
            if !manual {
                buffer_labels.push(parse_yaml!(entry.get("resource"), as_str, "resource"))
            }
        }

        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &entries,
        });

        if !manual {
            let indices: Vec<usize> = buffer_labels
                .into_iter()
                .map(|x| x.to_index(&mut context.renderer.shader_resources))
                .collect();

            let shader_resources: Vec<&BindingResource> = indices
                .iter()
                .map(|&idx| {
                    context.renderer.shader_resources.get(idx).ok_or(anyhow!(
                        "x_x :: shader resource non existent or whatever idk"
                    ))
                })
                .collect::<anyhow::Result<_>>()?;

            let resources: Vec<_> = shader_resources
                .iter()
                .enumerate()
                .map(|(i, x)| {
                    let resource = match entries[i] {
                        BindGroupLayoutEntry {
                            ty: BindingType::Sampler(_),
                            ..
                        } => {
                            let BindingResource::Texture(t) = x else {
                                panic!("x_x :: texture binding resource is not actually a Texture")
                            };
                            wgpu::BindingResource::Sampler(&t.sampler)
                        }
                        BindGroupLayoutEntry {
                            ty: BindingType::Texture { .. },
                            ..
                        } => {
                            let BindingResource::Texture(t) = x else {
                                panic!("x_x :: texture binding resource is not actually a Texture")
                            };
                            wgpu::BindingResource::TextureView(&t.view)
                        }
                        _ => x.binding(),
                    };
                    anyhow::Result::<BindGroupEntry>::Ok(BindGroupEntry {
                        binding: i as u32,
                        resource,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            bind_groups.push(Some(device.create_bind_group(&BindGroupDescriptor {
                label: Some(label),
                layout: &layout,
                entries: &resources,
            })));
        } else {
            bind_groups.push(None)
        }
        bind_group_layouts.push(layout);
    }
    let mut buffer_layouts = Vec::new();
    let mut offset = 0;
    // /let mut attrs_v: Vec<&'static mut [VertexAttribute]> = Vec::new();

    if let Some(byml) = root.get("vertex_buffers") {
        let buffers_yaml = byml.as_sequence().ok_or(anyhow!(
            "x_x :: invalid OMI yaml! field `vertex_buffers` is invalid"
        ))?;
        buffer_layouts = Vec::with_capacity(buffers_yaml.len());
        for i in buffers_yaml {
            let x = i.as_str().ok_or(anyhow!(
                "x_x :: invalid OMI yaml! invalid value `vertex_buffers`"
            ))?;

            let layout = mesh::vertex_from_name(x, offset)?;
            offset += layout.attrs.len() as u32;
            buffer_layouts.push(layout);
            // buffers.push(fns.1(p));
        }
    }
    if let Some(byml) = root.get("instance_buffers") {
        let buffers_yaml = byml.as_sequence().ok_or(anyhow!(
            "x_x :: invalid OMI yaml! field `instance_buffers` is invalid"
        ))?;
        buffer_layouts.reserve(buffers_yaml.len());
        for i in buffers_yaml {
            let x = i.as_str().ok_or(anyhow!(
                "x_x :: invalid OMI yaml! invalid value `instance_buffers`"
            ))?;
            let layout = mesh::instance_from_name(x, offset)?;
            offset += layout.attrs.len() as u32;
            buffer_layouts.push(layout);
        }
    }

    let buffers: Vec<_> = buffer_layouts
        .iter()
        .map(|x| VertexBufferLayout {
            step_mode: x.step_mode,
            array_stride: x.stride,
            attributes: &x.attrs,
        })
        .collect();

    let immediate_size = parse_yaml!(root.get("immediate_size"), as_u64, "immediate_size");
    dbg!(immediate_size);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(file_name),
        bind_group_layouts: &bind_group_layouts.iter().collect::<Vec<_>>(),
        immediate_size: immediate_size as u32,
    });

    let entry_points = unwrap_yaml!(root.get("entry_points"), "entry_points");
    let vert = unwrap_yaml!(entry_points.get("vertex"), "vertex");
    let frag = unwrap_yaml!(entry_points.get("fragment"), "fragment");
    let (vert_name, vert_fn) = (
        parse_yaml!(vert.get("module"), as_str, "vertex.module"),
        parse_yaml!(vert.get("function"), as_str, "vertex.function"),
    );
    let (frag_name, frag_fn) = (
        parse_yaml!(frag.get("module"), as_str, "fragment.module"),
        parse_yaml!(frag.get("function"), as_str, "fragment.function"),
    );

    let vert_text = load_string(vert_name, &context.resources_path).await?; //.expect("shader non existent in loading material");
    let frag_text = load_string(frag_name, &context.resources_path).await?; //.expect("shader non existent in loading material");

    let vert_descriptor = wgpu::ShaderModuleDescriptor {
        label: Some(vert_name),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&vert_text)),
    };
    let vert_shader = device.create_shader_module(vert_descriptor);
    let compare = if let Some(val) = root.get("always_on_top")
        && let Some(top) = val.as_bool()
        && top
    {
        wgpu::CompareFunction::Always
    } else {
        wgpu::CompareFunction::Less
    };
    let pipeline = if frag_name == vert_name {
        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(file_name),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &vert_shader,
                entry_point: Some(vert_fn),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            fragment: Some(FragmentState {
                module: &vert_shader,
                entry_point: Some(frag_fn),
                compilation_options: Default::default(),
                targets: &[Some(Material::screen_target(
                    context.renderer.config.format,
                ))],
            }),
            primitive: primitive_state,
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: compare,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        })
    } else {
        let frag_descriptor = wgpu::ShaderModuleDescriptor {
            label: Some(frag_name),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&frag_text)),
        };
        let frag_shader = device.create_shader_module(frag_descriptor);
        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(file_name),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &vert_shader,
                entry_point: Some(vert_fn),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            fragment: Some(FragmentState {
                module: &frag_shader,
                entry_point: Some(frag_fn),
                compilation_options: Default::default(),
                targets: &[Some(Material::screen_target(
                    context.renderer.config.format,
                ))],
            }),
            primitive: primitive_state,
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: compare,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        })
    };
    // dbg!(&pipeline);

    let material =
        Material::with_pipeline(file_name.to_owned(), pipeline, pipeline_layout, bind_groups);

    Ok(material)
}

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
    let omtl_text = load_string(&format!("{file_name}.omtl"), &context.resources_path)
        .await
        .with_context(|| format!("while loading omtl {}.omtl", file_name))?;
    let omtl_yaml: Value = serde_yaml::from_str(&omtl_text)?;
    let omi_names = parse_yaml!(omtl_yaml.get("materials"), as_sequence, "materials");
    let omi_index = parse_yaml!(omtl_yaml.get("objects"), as_sequence, "objects");
    let omis: Vec<&str> = omi_names.iter().map(|x| x.as_str().unwrap()).collect();
    let mut mids = Vec::with_capacity(omis.len());
    for i in omis {
        mids.push(
            load_material(i, context, None, None)
                .await
                .with_context(|| "while loading omi")?,
        );
    }
    let path = &context.resources_path;
    let (models, _) =
        tobj::load_obj_buf_async(&mut obj_reader, &tobj::GPU_LOAD_OPTIONS, |p| async move {
            //println!("{p}");
            let mat_text = load_string(&p, path).await.unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        })
        .await?;

    let d = omi_index
        .iter()
        .find_map(|x| {
            if let Some(default) = x.get("default") {
                default.as_u64()
            } else {
                None
            }
        })
        .expect("x_x :: invalid OMI yaml! missing or invalid default material index");

    let meshes = models
        .into_iter()
        .enumerate()
        .map(|(i, m)| {
            //println!("{m:#?}");
            let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| {
                    if m.mesh.normals.is_empty() {
                        mesh::ModelVertex {
                            position: [
                                m.mesh.positions[i * 3],
                                m.mesh.positions[i * 3 + 1],
                                m.mesh.positions[i * 3 + 2],
                            ],
                            tex_coords: [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]],
                            normal: [0.0, 0.0, 0.0],
                        }
                    } else {
                        mesh::ModelVertex {
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
            let s = omi_index.iter().find(|x| {
                if let Some(name) = x.get("name") {
                    let n = name
                        .as_str()
                        .expect("x_x :: invalid OMI yaml! invalid value for field `name`");
                    return n == m.name;
                };
                if let Some(index) = x.get("index") {
                    let n = index
                        .as_u64()
                        .expect("x_x :: invalid OMI yaml! invalid value for field `name`");
                    return n == i as u64;
                };
                false
            });
            let mid = if let Some(id) = s {
                id.get("material_index")
                    .expect("x_x :: invalid OMI yaml! missing `material_index` field")
                    .as_u64()
                    .expect("x_x :: invalid OMI yaml! invalid value for field `material_index`")
            } else {
                d
            };

            let mat = mids[mid as usize];

            mesh::InstancedMesh::new(
                mesh::Mesh::new(vertices, m.mesh.indices, mat, &mut context.renderer),
                vec![Instance {
                    position: Vec3::ZERO,
                    pivot: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ZERO,
                }],
                &mut context.renderer,
            )
        })
        .collect::<Vec<_>>();

    Ok(mesh::Model { meshes })
}

async fn binding_type(entry: &Value, context: &mut crate::Context) -> anyhow::Result<BindingType> {
    Ok(match parse_yaml!(entry.get("type"), as_str, "type") {
        "BUFFER" => {
            let buf = unwrap_yaml!(entry.get("buffer"), "buffer");
            let min_binding_size =
                unwrap_yaml!(buf.get("min_binding_size"), "min_binding_size").as_u64();
            BindingType::Buffer {
                ty: match parse_yaml!(buf.get("type"), as_str, "type") {
                    "UNIFORM" => wgpu::BufferBindingType::Uniform,
                    "STORAGE" => wgpu::BufferBindingType::Storage { read_only: false },
                    "READ_ONLY_STORAGE" => wgpu::BufferBindingType::Storage { read_only: true },
                    other => {
                        return Err(anyhow!(
                            "x_x :: invalid OMI yaml! invalid buffer `type` value: {}",
                            other
                        ));
                    }
                },
                has_dynamic_offset: parse_yaml!(
                    buf.get("has_dynamic_offset"),
                    as_bool,
                    "has_dynamic_offset"
                ),
                min_binding_size: match min_binding_size {
                    Some(x) => Some(NonZero::new(x).ok_or(anyhow!(
                        "x_x :: invalid OMI yaml! field `min_binding_size` is zero"
                    ))?),
                    None => None,
                },
            }
        }
        "SAMPLER" => {
            let _ = load_texture(
                parse_yaml!(entry.get("image_path"), as_str, "image_path"),
                context,
                Some(parse_yaml!(entry.get("resource"), as_str, "resource")),
            )
            .await;
            BindingType::Sampler(match parse_yaml!(entry.get("sampler"), as_str, "sampler") {
                "FILTERING" => wgpu::SamplerBindingType::Filtering,
                "COMPARISON" => wgpu::SamplerBindingType::Comparison,
                "NONFILTERING" => wgpu::SamplerBindingType::NonFiltering,
                other => {
                    return Err(anyhow!(
                        "x_x :: invalid OMI yaml! invalid `sampler` value: {}",
                        other
                    ));
                }
            })
        }
        "TEXTURE" => {
            let _ = load_texture(
                parse_yaml!(entry.get("image_path"), as_str, "image_path"),
                context,
                Some(parse_yaml!(entry.get("resource"), as_str, "resource")),
            )
            .await;
            let tex = unwrap_yaml!(entry.get("texture"), "texture");
            BindingType::Texture {
                sample_type: match parse_yaml!(tex.get("sampler_type"), as_str, "sampler_type") {
                    "FLOAT" => wgpu::TextureSampleType::Float {
                        filterable: parse_yaml!(tex.get("filterable"), as_bool, "filterable"),
                    },
                    "DEPTH" => wgpu::TextureSampleType::Depth,
                    "SINT" => wgpu::TextureSampleType::Sint,
                    "UINT" => wgpu::TextureSampleType::Uint,
                    other => {
                        return Err(anyhow!(
                            "x_x :: invalid OMI yaml! invalid `sample_type` value: {}",
                            other
                        ));
                    }
                },
                view_dimension: match parse_yaml!(
                    tex.get("view_dimension"),
                    as_str,
                    "view_dimension"
                ) {
                    "D1" => wgpu::TextureViewDimension::D1,
                    "D2" => wgpu::TextureViewDimension::D2,
                    "D2ARRAY" => wgpu::TextureViewDimension::D2Array,
                    "CUBE" => wgpu::TextureViewDimension::Cube,
                    "CUBEARRAY" => wgpu::TextureViewDimension::CubeArray,
                    "D3" => wgpu::TextureViewDimension::D3,
                    other => {
                        return Err(anyhow!(
                            "x_x :: invalid OMI yaml! invalid `view_dimension` value: {}",
                            other
                        ));
                    }
                },
                multisampled: parse_yaml!(tex.get("multisampled"), as_bool, "multisampled"),
            }
        }
        "STORAGE_TEXTURE" => {
            let st_tex = unwrap_yaml!(entry.get("storage_texture"), "storage_texture");
            BindingType::StorageTexture {
                access: match parse_yaml!(st_tex.get("access"), as_str, "access") {
                    "ATOMIC" => wgpu::StorageTextureAccess::Atomic,
                    "READ_ONLY" => wgpu::StorageTextureAccess::ReadOnly,
                    "READ_WRITE" => wgpu::StorageTextureAccess::ReadWrite,
                    "WRITE_ONLY" => wgpu::StorageTextureAccess::WriteOnly,
                    other => {
                        return Err(anyhow!(
                            "x_x :: invalid OMI yaml! invalid `access` value: {}",
                            other
                        ));
                    }
                },
                format: match parse_yaml!(st_tex.get("format"), as_str, "format") {
                    "R8UNORM" => wgpu::TextureFormat::R8Unorm,
                    "R8SNORM" => wgpu::TextureFormat::R8Snorm,
                    "R8UINT" => wgpu::TextureFormat::R8Uint,
                    "R8SINT" => wgpu::TextureFormat::R8Sint,
                    "R16UINT" => wgpu::TextureFormat::R16Uint,
                    "R16SINT" => wgpu::TextureFormat::R16Sint,
                    "R16UNORM" => wgpu::TextureFormat::R16Unorm,
                    "R16SNORM" => wgpu::TextureFormat::R16Snorm,
                    "R16FLOAT" => wgpu::TextureFormat::R16Float,
                    "RG8UNORM" => wgpu::TextureFormat::Rg8Unorm,
                    "RG8SNORM" => wgpu::TextureFormat::Rg8Snorm,
                    "RG8UINT" => wgpu::TextureFormat::Rg8Uint,
                    "RG8SINT" => wgpu::TextureFormat::Rg8Sint,
                    "R32UINT" => wgpu::TextureFormat::R32Uint,
                    "R32SINT" => wgpu::TextureFormat::R32Sint,
                    "R32FLOAT" => wgpu::TextureFormat::R32Float,
                    "RG16UINT" => wgpu::TextureFormat::Rg16Uint,
                    "RG16SINT" => wgpu::TextureFormat::Rg16Sint,
                    "RG16UNORM" => wgpu::TextureFormat::Rg16Unorm,
                    "RG16SNORM" => wgpu::TextureFormat::Rg16Snorm,
                    "RG16FLOAT" => wgpu::TextureFormat::Rg16Float,
                    "RGBA8UNORM" => wgpu::TextureFormat::Rgba8Unorm,
                    "RGBA8UNORMSRGB" => wgpu::TextureFormat::Rgba8UnormSrgb,
                    "RGBA8SNORM" => wgpu::TextureFormat::Rgba8Snorm,
                    "RGBA8UINT" => wgpu::TextureFormat::Rgba8Uint,
                    "RGBA8SINT" => wgpu::TextureFormat::Rgba8Sint,
                    "BGRA8UNORM" => wgpu::TextureFormat::Bgra8Unorm,
                    "BGRA8UNORMSRGB" => wgpu::TextureFormat::Bgra8UnormSrgb,
                    "RGB9E5UFLOAT" => wgpu::TextureFormat::Rgb9e5Ufloat,
                    "RGB10A2UINT" => wgpu::TextureFormat::Rgb10a2Uint,
                    "RGB10A2UNORM" => wgpu::TextureFormat::Rgb10a2Unorm,
                    "RG11B10UFLOAT" => wgpu::TextureFormat::Rg11b10Ufloat,
                    "R64UINT" => wgpu::TextureFormat::R64Uint,
                    "RG32UINT" => wgpu::TextureFormat::Rg32Uint,
                    "RG32SINT" => wgpu::TextureFormat::Rg32Sint,
                    "RG32FLOAT" => wgpu::TextureFormat::Rg32Float,
                    "RGBA16UINT" => wgpu::TextureFormat::Rgba16Uint,
                    "RGBA16SINT" => wgpu::TextureFormat::Rgba16Sint,
                    "RGBA16UNORM" => wgpu::TextureFormat::Rgba16Unorm,
                    "RGBA16SNORM" => wgpu::TextureFormat::Rgba16Snorm,
                    "RGBA16FLOAT" => wgpu::TextureFormat::Rgba16Float,
                    "RGBA32UINT" => wgpu::TextureFormat::Rgba32Uint,
                    "RGBA32SINT" => wgpu::TextureFormat::Rgba32Sint,
                    "RGBA32FLOAT" => wgpu::TextureFormat::Rgba32Float,
                    "STENCIL8" => wgpu::TextureFormat::Stencil8,
                    "DEPTH16UNORM" => wgpu::TextureFormat::Depth16Unorm,
                    "DEPTH24PLUS" => wgpu::TextureFormat::Depth24Plus,
                    "DEPTH24PLUSSTENCIL8" => wgpu::TextureFormat::Depth24PlusStencil8,
                    "DEPTH32FLOAT" => wgpu::TextureFormat::Depth32Float,
                    "DEPTH32FLOATSTENCIL8" => wgpu::TextureFormat::Depth32FloatStencil8,
                    "NV12" => wgpu::TextureFormat::NV12,
                    "BC1RGBAUNORM" => wgpu::TextureFormat::Bc1RgbaUnorm,
                    "BC1RGBAUNORMSRGB" => wgpu::TextureFormat::Bc1RgbaUnormSrgb,
                    "BC2RGBAUNORM" => wgpu::TextureFormat::Bc2RgbaUnorm,
                    "BC2RGBAUNORMSRGB" => wgpu::TextureFormat::Bc2RgbaUnormSrgb,
                    "BC3RGBAUNORM" => wgpu::TextureFormat::Bc3RgbaUnorm,
                    "BC3RGBAUNORMSRGB" => wgpu::TextureFormat::Bc3RgbaUnormSrgb,
                    "BC4RUNORM" => wgpu::TextureFormat::Bc4RUnorm,
                    "BC4RSNORM" => wgpu::TextureFormat::Bc4RSnorm,
                    "BC5RGUNORM" => wgpu::TextureFormat::Bc5RgUnorm,
                    "BC5RGSNORM" => wgpu::TextureFormat::Bc5RgSnorm,
                    "BC6HRGBUFLOAT" => wgpu::TextureFormat::Bc6hRgbUfloat,
                    "BC6HRGBFLOAT" => wgpu::TextureFormat::Bc6hRgbFloat,
                    "BC7RGBAUNORM" => wgpu::TextureFormat::Bc7RgbaUnorm,
                    "BC7RGBAUNORMSRGB" => wgpu::TextureFormat::Bc7RgbaUnormSrgb,
                    "ETC2RGB8UNORM" => wgpu::TextureFormat::Etc2Rgb8Unorm,
                    "ETC2RGB8UNORMSRGB" => wgpu::TextureFormat::Etc2Rgb8UnormSrgb,
                    "ETC2RGB8A1UNORM" => wgpu::TextureFormat::Etc2Rgb8A1Unorm,
                    "ETC2RGB8A1UNORMSRGB" => wgpu::TextureFormat::Etc2Rgb8A1UnormSrgb,
                    "ETC2RGBA8UNORM" => wgpu::TextureFormat::Etc2Rgba8Unorm,
                    "ETC2RGBA8UNORMSRGB" => wgpu::TextureFormat::Etc2Rgba8UnormSrgb,
                    "EACR11UNORM" => wgpu::TextureFormat::EacR11Unorm,
                    "EACR11SNORM" => wgpu::TextureFormat::EacR11Snorm,
                    "EACRG11UNORM" => wgpu::TextureFormat::EacRg11Unorm,
                    "EACRG11SNORM" => wgpu::TextureFormat::EacRg11Snorm,
                    "ASTC" => wgpu::TextureFormat::Astc {
                        block: match parse_yaml!(st_tex.get("format"), as_str, "format") {
                            "B4X4" => wgpu::AstcBlock::B4x4,
                            "B5X4" => wgpu::AstcBlock::B5x4,
                            "B5X5" => wgpu::AstcBlock::B5x5,
                            "B6X5" => wgpu::AstcBlock::B6x5,
                            "B6X6" => wgpu::AstcBlock::B6x6,
                            "B8X5" => wgpu::AstcBlock::B8x5,
                            "B8X6" => wgpu::AstcBlock::B8x6,
                            "B8X8" => wgpu::AstcBlock::B8x8,
                            "B10X5" => wgpu::AstcBlock::B10x5,
                            "B10X6" => wgpu::AstcBlock::B10x6,
                            "B10X8" => wgpu::AstcBlock::B10x8,
                            "B10X10" => wgpu::AstcBlock::B10x10,
                            "B12X10" => wgpu::AstcBlock::B12x10,
                            "B12X12" => wgpu::AstcBlock::B12x12,
                            other => {
                                return Err(anyhow!(
                                    "x_x :: invalid OMI yaml! invalid `block` value: {}",
                                    other
                                ));
                            }
                        },
                        channel: match parse_yaml!(st_tex.get("channel"), as_str, "channel") {
                            "HDR" => wgpu::AstcChannel::Hdr,
                            "UNORM" => wgpu::AstcChannel::Unorm,
                            "UNORM_SRGB" => wgpu::AstcChannel::UnormSrgb,
                            other => {
                                return Err(anyhow!(
                                    "x_x :: invalid OMI yaml! invalid `channel` value: {}",
                                    other
                                ));
                            }
                        },
                    },
                    other => {
                        return Err(anyhow!(
                            "x_x :: invalid OMI yaml! invalid `format` value: {}",
                            other
                        ));
                    }
                },
                view_dimension: match parse_yaml!(
                    st_tex.get("view_dimension"),
                    as_str,
                    "view_dimension"
                ) {
                    "D1" => wgpu::TextureViewDimension::D1,
                    "D2" => wgpu::TextureViewDimension::D2,
                    "D2ARRAY" => wgpu::TextureViewDimension::D2Array,
                    "CUBE" => wgpu::TextureViewDimension::Cube,
                    "CUBEARRAY" => wgpu::TextureViewDimension::CubeArray,
                    "D3" => wgpu::TextureViewDimension::D3,
                    other => {
                        return Err(anyhow!("invalid `view_dimension` value: {}", other));
                    }
                },
            }
        }
        "ACCELERATION_STRUCTURE" => BindingType::AccelerationStructure {
            vertex_return: parse_yaml!(entry.get("vertex_return"), as_bool, "vertex_return"),
        },
        other => {
            return Err(anyhow!(
                "x_x :: invalid OMI yaml! invalid bind group `type` value: {}",
                other
            ));
        }
    })
}
