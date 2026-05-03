use std::num::NonZero;

use anyhow::anyhow;
use serde_yaml::Value;
use wgpu::{BindGroupLayout, BindGroupLayoutEntry, BindingType, PrimitiveState, RenderPipeline};

use crate::{
    mesh::vertex,
    resources::{load_string, load_texture, texture},
};

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

// /// load a material from an .omi file
// ///
// /// if `resource_name` is omitted `file_name` is used to store the shader in the context.renderer;
// /// you should specify a resource name if you load the same shader twice with different options
// ///
// /// `primitive_state` is for advanced render pipeline config such as:
// /// - face culling
// /// - polygon mode (e.g. wireframe)
// /// - index format
// ///
// /// see wgpu::PrimitiveState
// pub async fn load_material(
//     file_name: &str,
//     context: &mut crate::Context,
//     resource_name: Option<&str>,
//     primitive_state: Option<PrimitiveState>,
// ) -> anyhow::Result<usize> {
//     let resource_name = match resource_name {
//         Some(name) => name,
//         None => file_name,
//     };
//     let primitive_state = match primitive_state {
//         Some(prim) => prim,
//         None => PrimitiveState {
//             cull_mode: Some(wgpu::Face::Back),
//             ..Default::default()
//         },
//     };
//
//     let id = resource_name.to_index(&mut context.renderer.materials);
//     if !context.renderer.materials.is_alive(id) {
//         let m = load_omi(file_name, context, primitive_state)
//             .await
//             .context(anyhow!("on shader: {}", resource_name))?;
//         context.renderer.materials.insert(resource_name, m);
//     }
//
//     Ok(id)
// }
/// such a pain to write AND badly done. the ultimate combination
/// idk why this is public but use it at your own discretion!
pub async fn load_pipeline(
    file_name: &str,
    context: &mut crate::Context,
    primitive_state: Option<PrimitiveState>,
) -> anyhow::Result<RenderPipeline> {
    // TODO: maybe use serde_yaml as intended. rather than manually parsing. idk
    let primitive_state = match primitive_state {
        Some(prim) => prim,
        None => PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
    };

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

    for group in bind_groups_yaml.iter() {
        let label = parse_yaml!(group.get("label"), as_str, "label");
        let entries_yaml = parse_yaml!(group.get("entries"), as_sequence, "entries");
        let mut entries = Vec::with_capacity(entries_yaml.len());
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
        }

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &entries,
        });

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

            let layout = vertex::vertex_from_name(x, offset)?;
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
            let layout = vertex::instance_from_name(x, offset)?;
            offset += layout.attrs.len() as u32;
            buffer_layouts.push(layout);
        }
    }

    let buffers: Vec<_> = buffer_layouts.iter().map(|x| x.desc()).collect();

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
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(file_name),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vert_shader,
                entry_point: Some(vert_fn),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &vert_shader,
                entry_point: Some(frag_fn),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: context.renderer.config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(file_name),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vert_shader,
                entry_point: Some(vert_fn),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &frag_shader,
                entry_point: Some(frag_fn),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: context.renderer.config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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

    Ok(pipeline)
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
            if let Some(path) = entry.get("image_path") {
                let _ = load_texture(
                    path.as_str().expect("x_x :: image_path is not a string"),
                    context,
                )
                .await;
            }
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
            if let Some(path) = entry.get("image_path") {
                let _ = load_texture(
                    path.as_str().expect("x_x :: image_path is not a string"),
                    context,
                )
                .await;
            }
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
