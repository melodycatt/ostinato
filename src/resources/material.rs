use wgpu::{
    BindGroup, ColorTargetState, Device, PipelineLayout, RenderPipeline, ShaderModule,
    TextureFormat, VertexBufferLayout,
};

use crate::resources::texture;

/// rendering material
/// FREAKY shit goes on in this impl dont even worry about it im not bothered to document it because i wrote it 50 billion years ago
#[derive(Debug)]
pub struct Material {
    pub bind_groups: Vec<Option<BindGroup>>,
    pub render_pipeline: RenderPipeline,
    // not sure we need this but its not hurting anyone
    pub pipeline_layout: PipelineLayout,
    pub name: String,
}

impl Material {
    /// you probably want `resources::load_shader()` instead
    pub fn new(
        name: String,
        MaterialPipelineInfo {
            shader_module,
            pipeline_layout,
            vertex_buffers,
            bind_groups,
            fragment_targets,
        }: MaterialPipelineInfo,
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
            multiview_mask: None,
            cache: None,
        });

        Self {
            //shader_modules: (shader_module, shader_module),
            //bind_group_layouts: bind_group_layouts.to_vec(),
            name,
            bind_groups,
            render_pipeline,
            pipeline_layout,
        }
    }
    /// you probably want `resources::load_shader()` instead
    pub fn with_pipeline(
        name: String,
        render_pipeline: RenderPipeline,
        pipeline_layout: PipelineLayout,
        bind_groups: Vec<Option<BindGroup>>,
        //bind_group_layouts: &[Arc<BindGroupLayout>],
        //device: &Device,
    ) -> Self {
        //println!("{:?}", pipeline_layout);

        Self {
            //shader_modules,
            //bind_group_layouts: bind_group_layouts.to_vec(),
            name,
            bind_groups,
            render_pipeline,
            pipeline_layout,
        }
    }
    /// you probably want `resources::load_shader()` instead
    pub fn new_no_stencil(
        name: String,
        MaterialPipelineInfo {
            shader_module,
            pipeline_layout,
            vertex_buffers,
            bind_groups,
            fragment_targets,
        }: MaterialPipelineInfo,
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        Self {
            //shader_module,
            //bind_group_layouts: bind_group_layouts.to_vec(),
            name,
            bind_groups,
            render_pipeline,
            pipeline_layout,
        }
    }

    /// fucking idk dudeeeeee
    pub fn screen_target(format: TextureFormat) -> ColorTargetState {
        wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        }
    }
    //pub fn init()
}

pub struct MaterialPipelineInfo<'a> {
    pub shader_module: ShaderModule,
    pub bind_groups: Vec<Option<BindGroup>>,
    pub pipeline_layout: PipelineLayout,
    pub vertex_buffers: &'a [VertexBufferLayout<'a>],
    pub fragment_targets: &'a [Option<ColorTargetState>],
}
