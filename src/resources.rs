use std::{any::Any, cell::RefMut, fmt::Debug, io::{BufReader, Cursor}, num::NonZero, sync::Arc};

use anyhow::{anyhow, Error};
use serde_yaml::Value;
use wgpu::{BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, FragmentState, MultisampleState, PrimitiveState, RenderPipelineDescriptor, VertexState};

extern crate alloc;

use crate::{mesh::{self, Material}, texture, ResourceId, State};

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

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
    #[cfg(target_arch = "wasm32")]
    let txt = {
        let url = format_url(file_name);
        reqwest::get(url).await?.text().await?
    };
    #[cfg(not(target_arch = "wasm32"))]
    let txt = {
        let path = std::path::Path::new(env!("OUT_DIR"))
            .join("res")
            .join(file_name);
        println!("{:?}", path);
        std::fs::read_to_string(path)?
    };

    Ok(txt)
}

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    let data = {
        let url = format_url(file_name);
        reqwest::get(url).await?.bytes().await?.to_vec()
    };
    #[cfg(not(target_arch = "wasm32"))]
    let data = {
        let path = std::path::Path::new(env!("OUT_DIR"))
            .join("res")
            .join(file_name);
        std::fs::read(path)?
    };

    Ok(data)
}


pub async fn load_texture(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<texture::Texture> {
    let data = load_binary(file_name).await?;
    texture::Texture::from_bytes(device, queue, &data, file_name)
}

macro_rules! parse_yaml {
    ($val:expr, $as:ident, $name:expr) => {
        $val
            .ok_or(anyhow!("x_x :: invalid OMI yaml! missing `{}` in bind group", $name))?.$as()
            .ok_or(anyhow!("x_x :: invalid OMI yaml! field `{}` is invalid", $name))?
    };
}
macro_rules! unwrap_yaml {
    ($val:expr, $name:expr) => {
        $val
            .ok_or(anyhow!("x_x :: invalid OMI yaml! missing `{}` in bind group", $name))?
    };
}

pub async fn load_shader(
    file_name: &str,
    state: &mut State
) -> anyhow::Result<ResourceId> {
    let f = format!("material::{file_name}");
    let id: ResourceId = f.into();
    let existing = state.resources.contains_key(&id.key().unwrap());
    let device = state.graphics().device.clone();
    if existing {
        return Ok(id);
    } else {
        let omi_text = load_string(&format!("{file_name}.omi")).await?;
        let root: Value = serde_yaml::from_str(&omi_text)?;

        let bind_groups_yaml = parse_yaml!(root.get("bind_groups"), as_sequence, "bind_groups");
            // .ok_or(anyhow!("x_x :: invalid OMI yaml! missing bind_groups"))?.as_sequence()
            // .ok_or(anyhow!("x_x :: invalid OMI yaml! field bind_groups is invalid"))?;
        let mut bind_group_layouts = Vec::with_capacity(bind_groups_yaml.len());
        let mut bind_groups = Vec::with_capacity(bind_groups_yaml.len());
        for group in bind_groups_yaml {
            if let Some(preset) = group.get("preset") {
                match preset.as_str().ok_or(anyhow!("x_x :: invalid bind group preset!"))? {
                    "CAMERA" => {
                        let layout: &BindGroupLayout = &state.camera().bind_group_layout;
                        let cam = state.camera();
                        let binding = crate::camera::Camera::binding(&*cam)?;
                        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                            layout: layout,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: binding,
                                }   
                            ],    
                            label: Some("CAMERA"),
                        });
                        bind_groups.push(camera_bind_group);
                        bind_group_layouts.push(Arc::clone(&state.camera().bind_group_layout));
                    }
                    other => {return Err(anyhow!("invalid bind group preset: {}", other))}
                }
                println!("why dont we do this")
            } else {
                let label = parse_yaml!(group.get("label"), as_str, "label");
                let entries_yaml = parse_yaml!(group.get("entries"), as_sequence, "entries");
                let mut entries = Vec::with_capacity(entries_yaml.len());
                let mut resource_labels = Vec::with_capacity(entries_yaml.len());
                for e in 0..entries_yaml.len() {
                    let entry = &entries_yaml[e];
                    let vis = parse_yaml!(entry.get("visibility"), as_sequence, "visibility")
                        .iter()
                        .map(|value| {
                            match value.as_str() {
                                Some("FRAGMENT") => Ok(wgpu::ShaderStages::FRAGMENT),
                                Some("COMPUTE") => Ok(wgpu::ShaderStages::COMPUTE),
                                Some("MESH") => Ok(wgpu::ShaderStages::MESH),
                                Some("VERTEX") => Ok(wgpu::ShaderStages::VERTEX),
                                Some("NONE") => Ok(wgpu::ShaderStages::NONE),
                                Some("TASK") => Ok(wgpu::ShaderStages::TASK),
                                Some("VERTEX_FRAGMENT") => Ok(wgpu::ShaderStages::VERTEX_FRAGMENT),
                                Some(v) => Err(anyhow!("x_x :: invalid OMI yaml! invalid `visibility` value: {v}")),
                                None => Err(anyhow!("x_x :: invalid OMI yaml! `visibility` must be a string")),
                            }
                        })
                        .try_fold(wgpu::ShaderStages::NONE, |acc, x| x.map(|s| acc | s))?;
                    let group_type = match parse_yaml!(entry.get("type"), as_str, "type") {
                        "BUFFER" => {
                            let buf = unwrap_yaml!(entry.get("buffer"), "buffer");
                            let min_binding_size = unwrap_yaml!(buf.get("min_binding_size"), "min_binding_size").as_u64();
                            BindingType::Buffer { 
                                ty: match parse_yaml!(buf.get("type"), as_str, "type") {
                                    "UNIFORM" => wgpu::BufferBindingType::Uniform,
                                    "STORAGE" => wgpu::BufferBindingType::Storage { read_only: false },
                                    "READ_ONLY_STORAGE" => wgpu::BufferBindingType::Storage { read_only: true },
                                    other => return Err(anyhow!("x_x :: invalid OMI yaml! invalid buffer `type` value: {}", other)),
                                },
                                has_dynamic_offset: parse_yaml!(buf.get("has_dynamic_offset"), as_bool, "has_dynamic_offset"), 
                                min_binding_size: match min_binding_size {
                                    Some(x) => Some(NonZero::new(x).ok_or(anyhow!("x_x :: invalid OMI yaml! field `min_binding_size` is zero"))?),
                                    None => None
                                },
                            }
                        },
                        "SAMPLER" => {
                            BindingType::Sampler(match parse_yaml!(entry.get("sampler"), as_str, "sampler") {
                                "FILTERING" => wgpu::SamplerBindingType::Filtering,
                                "COMPARISON" => wgpu::SamplerBindingType::Comparison,
                                "NONFILTERING" => wgpu::SamplerBindingType::NonFiltering, 
                                other => return Err(anyhow!("x_x :: invalid OMI yaml! invalid `sampler` value: {}", other)),
                            })
                        },
                        "TEXTURE" => {
                            let tex = unwrap_yaml!(entry.get("texture"), "texture");
                            BindingType::Texture { 
                                sample_type: match parse_yaml!(tex.get("sampler_type"), as_str, "sampler_type") {
                                    "FLOAT" => wgpu::TextureSampleType::Float { filterable: parse_yaml!(tex.get("filterable"), as_bool, "filterable") },
                                    "DEPTH" => wgpu::TextureSampleType::Depth,
                                    "SINT" => wgpu::TextureSampleType::Sint,
                                    "UINT" => wgpu::TextureSampleType::Uint,   
                                    other => return Err(anyhow!("x_x :: invalid OMI yaml! invalid `sample_type` value: {}", other)),
                                }, 
                                view_dimension: match parse_yaml!(tex.get("view_dimension"), as_str, "view_dimension") {
                                    "D1" => wgpu::TextureViewDimension::D1,
                                    "D2" => wgpu::TextureViewDimension::D2,
                                    "D2ARRAY" => wgpu::TextureViewDimension::D2Array,
                                    "CUBE" => wgpu::TextureViewDimension::Cube,
                                    "CUBEARRAY" => wgpu::TextureViewDimension::CubeArray,
                                    "D3" => wgpu::TextureViewDimension::D3,
                                    other => return Err(anyhow!("x_x :: invalid OMI yaml! invalid `view_dimension` value: {}", other)),
                                }, 
                                multisampled: parse_yaml!(tex.get("multisampled"), as_bool, "multisampled")
                            }
                        },
                        "STORAGE_TEXTURE" => {
                            let st_tex = unwrap_yaml!(entry.get("storage_texture"), "storage_texture");
                            BindingType::StorageTexture { 
                                access: match parse_yaml!(st_tex.get("access"), as_str, "access") {
                                    "ATOMIC" => wgpu::StorageTextureAccess::Atomic,
                                    "READ_ONLY" => wgpu::StorageTextureAccess::ReadOnly,
                                    "READ_WRITE" => wgpu::StorageTextureAccess::ReadWrite,
                                    "WRITE_ONLY" => wgpu::StorageTextureAccess::WriteOnly,
                                    other => return Err(anyhow!("x_x :: invalid OMI yaml! invalid `access` value: {}", other)),
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
                                            other => return Err(anyhow!("x_x :: invalid OMI yaml! invalid `block` value: {}", other))
                                        }, channel: match parse_yaml!(st_tex.get("channel"), as_str, "channel") {
                                            "HDR" => wgpu::AstcChannel::Hdr,
                                            "UNORM" => wgpu::AstcChannel::Unorm,
                                            "UNORM_SRGB" => wgpu::AstcChannel::UnormSrgb,
                                            other => return Err(anyhow!("x_x :: invalid OMI yaml! invalid `channel` value: {}", other))
                                        } },
                                    other => return Err(anyhow!("x_x :: invalid OMI yaml! invalid `format` value: {}", other)),
                                },
                                view_dimension: match parse_yaml!(st_tex.get("view_dimension"), as_str, "view_dimension") {
                                    "D1" => wgpu::TextureViewDimension::D1,
                                    "D2" => wgpu::TextureViewDimension::D2,
                                    "D2ARRAY" => wgpu::TextureViewDimension::D2Array,
                                    "CUBE" => wgpu::TextureViewDimension::Cube,
                                    "CUBEARRAY" => wgpu::TextureViewDimension::CubeArray,
                                    "D3" => wgpu::TextureViewDimension::D3,
                                    other => return Err(anyhow!("invalid `view_dimension` value: {}", other)),
                                }, 
                            }
                        },
                        "ACCELERATION_STRUCTURE" => {
                            BindingType::AccelerationStructure { vertex_return: parse_yaml!(entry.get("vertex_return"), as_bool, "vertex_return") }
                        },
                        other => return Err(anyhow!("x_x :: invalid OMI yaml! invalid bind group `type` value: {}", other)),
                    };
                    let count_parse = unwrap_yaml!(entry.get("count"), "count").as_u64();
                    let count = match count_parse {
                        Some(x) => Some(NonZero::new(x as u32).ok_or(anyhow!("x_x :: invalid OMI yaml! field `min_binding_size` is zero"))?),
                        None => None
                    };
                    entries.push(BindGroupLayoutEntry {
                        binding: e as u32,
                        visibility: vis,
                        ty: group_type,
                        count
                    });
                    resource_labels.push(parse_yaml!(entry.get("resource"), as_str, "resource"))
                }
                let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some(label),
                    entries: &entries
                });
                println!("{:#?}", entries);
                let binding = resource_labels.into_iter()
                    .map(|x| { println!("{:?} {:?}", x, Into::<ResourceId>::into(&*x)); state.get_resource_mut(&(*x).into()).map_err(|e| anyhow!("x_x :: invalid OMI yaml! when parsing {x}: {}", e.to_string())) })
                    .collect::<Result<Vec<RefMut<Box<dyn Resource + 'static>>>, _>>()?;
                let resources: Vec<_> = binding
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        //x.try_borrow_mut().map_err(|_| anyhow!("x_x :: tried to load a material with a resource that was already borrowed mutably in one of its bind groups"))?;
                        //let raw = *x as *const Box<dyn Resource>;
                       //println!()
                        let resource = x;
                        Ok::<_, anyhow::Error>(BindGroupEntry {
                            binding: i as u32,
                            resource: resource.binding()?
                        })
                    })
                    .collect::<Result<Vec<BindGroupEntry<'_>>, _>>()?;
                bind_groups.push(device.create_bind_group(&BindGroupDescriptor {
                    label: Some(label),
                    layout: &layout,
                    entries: &resources
                }));
                bind_group_layouts.push(Arc::new(layout));
            }

        }
        let mut buffers = Vec::new();
        if let Some(byml) = root.get("buffers") {
            let buffers_yaml = byml.as_sequence().ok_or(anyhow!("x_x :: invalid OMI yaml! field `buffers` is invalid"))?;
            buffers = Vec::with_capacity(buffers_yaml.len());
            for i in buffers_yaml {
                let x = i.as_str().ok_or(anyhow!("x_x :: invalid OMI yaml! invalid value `buffers`"))?;
                buffers.push(mesh::desc_from_name(x)?);
            }
        }

        let layouts: Vec<_> = bind_group_layouts.iter().map(|arc| arc.as_ref()).collect();
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(file_name),
            bind_group_layouts: &layouts,
            push_constant_ranges: &[],
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

        let vert_text = load_string(&vert_name).await?;//.expect("shader non existent in loading material");
        let frag_text = load_string(&frag_name).await?;//.expect("shader non existent in loading material");
        println!("{}", format!("{file_name}.wgsl"));
        let vert_descriptor = wgpu::ShaderModuleDescriptor {
            label: Some(vert_name),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&vert_text)),
        };
        let vert_shader = device.create_shader_module(vert_descriptor);
        let pipeline = if frag_name == vert_name {
            device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(file_name),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &vert_shader,
                    entry_point: Some(vert_fn),
                    compilation_options: Default::default(),
                    buffers: &buffers
                },
                fragment: Some(FragmentState {
                    module: &vert_shader,
                    entry_point: Some(frag_fn),
                    compilation_options: Default::default(),
                    targets: &[Some(mesh::Material::screen_target(state.graphics().config.format))]
                }),
                primitive: PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: texture::Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None
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
                    buffers: &buffers
                },
                fragment: Some(FragmentState {
                    module: &frag_shader,
                    entry_point: Some(frag_fn),
                    compilation_options: Default::default(),
                    targets: &[Some(mesh::Material::screen_target(state.graphics().config.format))]
                }),
                primitive: PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: texture::Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None
            })
        };
        // include_wgsl!()

        //println!("{:?}", pipeline_layout);
        //println!("{:?}", layouts);


        let material = Material::with_pipeline(pipeline, pipeline_layout, bind_groups);
        state.create_resource(format!("material::{file_name}").into(), material);

        return Ok(format!("material::{file_name}").into())
    }
}

/// this trait is blanket implemented to return an error
/// if you really really need a custom type to implement `binding`
/// (which in most cases the default types will be enough)
/// use #![feature(min_specialization)] on nightly to override the blanket impl
pub trait Resource: Any+Debug {
    /*fn as_any(&self) -> &dyn std::any::Any;

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;*/
    /// may not actually be implemented
    /// used when loading a shader, we need to assume the resource can create a binding
    /// however, we dont know the type, so we dont know how, which is why a trait is needed
    /// 
    /// keep in mind, we are *assuming* the resource can create a binding; not all resources need to do this
    fn binding<'a>(&'a self) -> anyhow::Result<BindingResource<'a>> {
        Err(anyhow!("x_x :: tried to get a BindingResource from an incompatible Resource (only certain types can become BindingResources)"))
    }
}

/*impl<T: Any+Debug> Resource for T {
    /*default fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    default fn as_any(&self) -> &dyn std::any::Any {
        self
    }*/
    default fn binding<'a>(&'a self) -> anyhow::Result<BindingResource<'a>>  {
        Err(anyhow!("x_x :: tried to get a BindingResource from an incompatible Resource (only certain types can become BindingResources)"))
    }
}*/

impl Resource for wgpu::Buffer {
    // default fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
    //     self
    // }
    // fn as_any(&self) -> &dyn std::any::Any {
    //     self
    // }
    fn binding<'a>(&'a self) -> anyhow::Result<BindingResource<'a>> {
        Ok(self.as_entire_binding())
    }
}
impl Resource for std::time::Instant {}
impl Resource for Arc<winit::window::Window> {}

/// .omi (ostinato material info) is info on how to construct a material from
/// .omtl (ostinto mtl) is a bundle of material infos

pub async fn load_model(
    file_name: &str,
    //device: &wgpu::Device,
    //queue: &wgpu::Queue,
    //layout: &wgpu::BindGroupLayout,
    state: &mut State
) -> anyhow::Result<mesh::Model> {
    //let device = state.graphics().device.clone();
    //let queue = &wgpu.queue;

    let obj_text = load_string(&format!("{file_name}.obj")).await?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);
    let omtl_text = load_string(&format!("{file_name}.omtl")).await?;
    let omtl_yaml: Value = serde_yaml::from_str(&omtl_text)?;
    println!("!!");
    let omi_names = parse_yaml!(omtl_yaml.get("materials"), as_sequence, "materials");
    let omi_index = parse_yaml!(omtl_yaml.get("objects"), as_sequence, "materials");
    let omis: Vec<&str> = omi_names.iter().map(|x| x.as_str().unwrap()).collect();
    let mut mids = Vec::with_capacity(omis.len());
    for i in omis {
        mids.push(load_shader(i, state).await?);
    }

    let (models, _) = tobj::load_obj_buf_async(
        &mut obj_reader,
        &tobj::GPU_LOAD_OPTIONS,
        |p| async move {
            //println!("{p}");
            let mat_text = load_string(&p).await.unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    )
    .await?;

    //let mut materials = Vec::new();
    /*for m in obj_materials? {
        let diffuse_texture = load_texture(&m.diffuse_texture, device, queue).await?;
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: None,
        });

        materials.push(mesh::Material {
            name: m.name,
            diffuse_texture,
            bind_group,
        })
    }*/

    let meshes = models
        .into_iter()
        .map(|m| {
            //println!("{m:#?}");
                let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| {
                    if m.mesh.normals.is_empty(){
                        mesh::ModelVertex {
                            position: [
                                m.mesh.positions[i * 3],
                                m.mesh.positions[i * 3 + 1],
                                m.mesh.positions[i * 3 + 2],
                            ],
                            tex_coords: [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]],
                            normal: [0.0, 0.0, 0.0],
                        }
                    }else{
                        mesh::ModelVertex {
                            position: [
                                m.mesh.positions[i * 3],
                                m.mesh.positions[i * 3 + 1],
                                m.mesh.positions[i * 3 + 2],
                            ],
                            tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
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

            mesh::Mesh::new(vertices, m.mesh.indices, mids[m.mesh.material_id.unwrap_or(0)], Some(file_name.to_string()), state)
        })
        .collect::<Vec<_>>();

    Ok(mesh::Model { meshes })
}