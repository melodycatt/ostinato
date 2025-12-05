use derive_resource::Resource;
use wgpu::{BindGroup, ColorTargetState, Device, PipelineLayout, RenderPipeline, ShaderModule, TextureFormat, VertexBufferLayout};

use crate::{resources::Resource, texture};

#[derive(Debug, Resource)]
pub struct Material {
    //not sure we actually need to store the shader_modules
    //pub shader_modules: (ShaderModule, ShaderModule),
    pub shared_bind_groups: Vec<usize>,
    pub bind_groups: Vec<BindGroup>,
    pub render_pipeline: RenderPipeline,
    //not sure we need this either but its not hurting anyone
    pub pipeline_layout: PipelineLayout,
    pub name: String
}

impl Material {
    pub fn new(
        name: String,
        shader_module: ShaderModule,
        shared_bind_groups: Vec<usize>,
        bind_groups: Vec<BindGroup>,
        //bind_group_layouts: &[Arc<BindGroupLayout>],
        pipeline_layout: PipelineLayout,
        vertex_buffers: &[VertexBufferLayout],
        fragment_targets: &[Option<ColorTargetState>],
        device: &Device,
    ) -> Self {
        println!("{:?}", pipeline_layout);

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shader Render Pipeline"),
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
            //shader_modules: (shader_module, shader_module),
            //bind_group_layouts: bind_group_layouts.to_vec(),
            name,
            bind_groups,
            shared_bind_groups,
            render_pipeline,
            pipeline_layout,
        }
    }
    pub fn with_pipeline(
        name: String,
        render_pipeline: RenderPipeline,
        pipeline_layout: PipelineLayout,
        shared_bind_groups: Vec<usize>,
        bind_groups: Vec<BindGroup>,
        //bind_group_layouts: &[Arc<BindGroupLayout>],
        //device: &Device,
    ) -> Self {
        //println!("{:?}", pipeline_layout);

        Self {
            //shader_modules,
            //bind_group_layouts: bind_group_layouts.to_vec(),
            name,
            bind_groups,
            shared_bind_groups,
            render_pipeline,
            pipeline_layout,
        }
    }
    pub fn new_no_stencil(
        name: String,
        shader_module: ShaderModule,
        shared_bind_groups: Vec<usize>,
        bind_groups: Vec<BindGroup>,
        //bind_group_layouts: &[Arc<BindGroupLayout>],
        pipeline_layout: PipelineLayout,
        vertex_buffers: &[VertexBufferLayout],
        fragment_targets: &[Option<ColorTargetState>],
        device: &Device,
    ) -> Self {
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shader Render Pipeline"),
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
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil:None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            //shader_module,
            //bind_group_layouts: bind_group_layouts.to_vec(),
            name,
            bind_groups,
            shared_bind_groups,
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