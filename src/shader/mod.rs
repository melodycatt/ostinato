use std::sync::Arc;

use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, ColorTargetState, Device, PipelineLayout, RenderPass, RenderPipeline, ShaderModule, TextureFormat, VertexBufferLayout};

use crate::texture;

pub struct Shader {
    pub shader_module: ShaderModule,
    //pub bind_group_layouts: Vec<Arc<BindGroupLayout>>,
    pub bind_groups: Vec<BindGroup>,
    pub render_pipeline: RenderPipeline,
    pub pipeline_layout: PipelineLayout
}

impl Shader {
    pub fn new(
        shader_module: ShaderModule,
        bind_groups: Vec<BindGroup>,
        //bind_group_layouts: &[Arc<BindGroupLayout>],
        pipeline_layout: PipelineLayout,
        vertex_buffers: &[VertexBufferLayout],
        fragment_targets: &[Option<ColorTargetState>],
        device: &Device,
    ) -> Self {
        //let mut bind_groups = Vec::with_capacity(bind_group_layouts.len());
        /*for i in 0..bind_group_layouts.len() {
            
            /*let mut entries: Vec<_> = Vec::with_capacity(bind_group_resources[i].len());
            for j in 0..bind_group_resources[i].len() {
                entries.push(BindGroupEntry {
                    binding: j as u32,
                    resource: bind_group_resources[i][j],
                });
            }*/
            let entries: Vec<_> = bind_group_resources[i].iter().enumerate().map(|(i, x)| {x.binding = i as u32; x}).collect();

            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                layout: &bind_group_layouts[i],
                entries: entries.as_slice(),
                label: Some("shader_bind_group"),
            });
            bind_groups.push(bind_group);
        }*/

        /*let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &bind_group_layouts.iter().map(|l| &**l).collect::<Vec<_>>(),
            push_constant_ranges: &[],
        });*/

        //println!("{:?}", &bind_group_layouts.iter().map(|l| &**l).collect::<Vec<_>>());

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: vertex_buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: fragment_targets,
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
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
            cache: None,
        });

        Self {
            shader_module,
            //bind_group_layouts: bind_group_layouts.to_vec(),
            bind_groups,
            render_pipeline,
            pipeline_layout,
        }
    }

    pub fn screen_target(format: TextureFormat) -> ColorTargetState {
        wgpu::ColorTargetState {
            format: format,
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent::REPLACE,
                alpha: wgpu::BlendComponent::REPLACE,
            }),    
            write_mask: wgpu::ColorWrites::ALL,
        }
    }
    //pub fn init()
}