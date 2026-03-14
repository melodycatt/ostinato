use std::num::NonZeroU32;

use smallvec::SmallVec;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, CommandEncoder, FragmentState, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, RenderPipeline,
    RenderPipelineDescriptor, ShaderStages, VertexBufferLayout, VertexState,
};

use crate::Context;
use crate::camera::CameraUniform;
use crate::renderer::InstanceRaw;
use crate::resources::{BufferLayout, ModelVertex, load_shader};
use crate::{prelude::Renderer, resources::BindingResource};

// mod sealed {
//     pub(super) trait Sealed {}
// }
pub trait LazyPass {
    fn get(context: &mut Context) -> &mut PassData;
}
pub trait LazyMaterial {
    fn get(context: &mut Context) -> &mut MaterialData;
}
impl<T: RenderPass> LazyPass for T {
    fn get(context: &mut Context) -> &mut PassData {
        context.get_pass_data::<Self>()
    }
}
impl<T: MaterialType> LazyMaterial for T {
    fn get(context: &mut Context) -> &mut MaterialData {
        context.get_material_data::<Self>()
    }
}
pub trait BindGroupGenerator: 'static {
    fn bind_group(
        layout: &BindGroupLayout,
        resources: &[BindingResource],
        context: &mut Context,
    ) -> BindGroup {
        context
            .renderer
            .device
            .create_bind_group(&BindGroupDescriptor {
                label: None,
                entries: &resources
                    .iter()
                    .enumerate()
                    .map(|(i, r)| BindGroupEntry {
                        binding: i as u32,
                        resource: r.binding(),
                    })
                    .collect::<Vec<_>>(),
                layout,
            })
    }
    fn bind_group_layout(context: &mut Context) -> BindGroupLayout;
    fn resources(context: &mut Context) -> Vec<BindingResource>;
}
pub trait RenderPass: BindGroupGenerator {
    type Immediates: bytemuck::Pod;
    const IMMEDIATES_END: u32 = std::mem::size_of::<Self::Immediates>() as u32;
    fn set_immediates(pass: &mut wgpu::RenderPass<'_>, immediates: &Self::Immediates) {
        pass.set_immediates(0, bytemuck::bytes_of(immediates))
    }
    // type ImmediateTypes: ...;
    fn render_pass<'b>(
        renderer: &mut Renderer,
        encoder: &'b mut CommandEncoder,
    ) -> anyhow::Result<(wgpu::SurfaceTexture, wgpu::RenderPass<'b>), wgpu::SurfaceError> {
        let surface_texture = renderer.surface.get_current_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.,
                        g: 0.,
                        b: 0.,
                        a: 1.,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &renderer.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            multiview_mask: None,
            timestamp_writes: None,
        });

        Ok((surface_texture, pass))
    }
    fn fragment_targets<'a>(
        renderer: &'a mut Renderer,
    ) -> SmallVec<[Option<wgpu::ColorTargetState>; 4]> {
        smallvec::smallvec![Some(wgpu::ColorTargetState {
            format: renderer.config.format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::all(),
        })]
    }
}

pub struct GeometryPass {}
impl RenderPass for GeometryPass {
    type Immediates = InstanceRaw;
}
impl BindGroupGenerator for GeometryPass {
    fn resources(context: &mut Context) -> Vec<BindingResource> {
        vec![
            context
                .renderer
                .device
                .create_buffer(&wgpu::BufferDescriptor {
                    label: Some("shared camera buffer"),
                    size: std::mem::size_of::<CameraUniform>() as u64,
                    mapped_at_creation: false,
                    usage: wgpu::BufferUsages::COPY_DST.union(wgpu::BufferUsages::UNIFORM),
                })
                .into(),
        ]
    }
    fn bind_group_layout(context: &mut Context) -> BindGroupLayout {
        context
            .renderer
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }
}

pub(crate) struct PassData {
    pub bind_group: BindGroup,
    pub layout: BindGroupLayout,
    pub resources: Vec<BindingResource>,
}
impl PassData {
    /// dont ue this dont use this dont use this
    pub fn of<T: RenderPass>(context: &mut Context) -> Self {
        let resources = T::resources(context);
        let layout = T::bind_group_layout(context);
        Self {
            bind_group: T::bind_group(&layout, &resources, context),
            layout: layout,
            resources: resources,
        }
    }
}
pub(crate) struct MaterialData {
    pub bind_group: BindGroup,
    pub resources: Vec<BindingResource>,
    pub pipeline: RenderPipeline,
}
impl MaterialData {
    pub fn of<T: MaterialType>(context: &mut Context) -> Self {
        let resources = T::resources(context);
        let layout = T::bind_group_layout(context);

        let bind_group = T::bind_group(&layout, &resources, context);
        Self {
            bind_group,
            resources,
            pipeline: T::pipeline(layout, context),
        }
    }
}

// TODO: make BUFFERS better: currently people need to use nightly feature gate to make a new
// material actually now i type that its really bad so FIGURE IT OUT
/// N is the number of extra bind groups
pub trait MaterialType: BindGroupGenerator {
    /// TODO: err actually theres two types of immediates here ones that come from the material and
    /// ones that come from a mesh and stuff. dont know how to make this work
    type Immediates: bytemuck::Pod;
    type Pass: RenderPass;
    /// TODO: auto pipeline layout?
    const CONFIG: MaterialConfig;
    const IMMEDIATE_SIZE: u32 = sum_immediate_sizes::<Self>();
    const IMMEDIATE_OFFSET: u32 = Self::Pass::IMMEDIATES_END;

    fn pipeline(layout: BindGroupLayout, context: &mut Context) -> RenderPipeline {
        let device = context.renderer.device.clone();
        let shaders = Self::CONFIG.shader_config;
        let vertex_mod = load_shader(shaders.0.shader_path, context).unwrap();
        let fragment_mod = if Self::CONFIG.shader_config.0.shader_path
            == Self::CONFIG.shader_config.1.shader_path
        {
            None
        } else {
            Some(load_shader(shaders.1.shader_path, context).unwrap())
        };

        // if shaders.0
        let vbgls = Self::create_bind_group_layouts(&device);
        let mut full_bgls = Vec::with_capacity(2 + vbgls.len());
        full_bgls.push(&context.get_pass_data::<Self::Pass>().layout);
        full_bgls.push(&layout);
        full_bgls.extend(vbgls.iter());

        let playout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &full_bgls,
            immediate_size: Self::IMMEDIATE_SIZE,
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            // TODO
            label: None,
            layout: Some(&playout),
            vertex: VertexState {
                module: &vertex_mod,
                entry_point: shaders.0.entry_point,
                compilation_options: shaders.0.compilation_options,
                buffers: Self::CONFIG.buffers,
            },
            primitive: Self::CONFIG.primitive,
            depth_stencil: Self::CONFIG.depth_stencil,
            multisample: Self::CONFIG.multisample,
            fragment: Some(FragmentState {
                module: fragment_mod.as_ref().unwrap_or(&vertex_mod),
                entry_point: shaders.1.entry_point,
                compilation_options: shaders.1.compilation_options,
                targets: &Self::fragment_targets(&mut context.renderer),
            }),
            multiview_mask: Self::CONFIG.multiview_mask,
            // TODO: cache support?
            cache: None,
        })
    }

    /// set immediates from external source
    fn set_immediates(pass: &mut wgpu::RenderPass<'_>, immediates: &Self::Immediates) {
        pass.set_immediates(Self::Pass::IMMEDIATES_END, bytemuck::bytes_of(immediates))
    }
    /// set any individual bind groups or immediates here - things related to data stored on
    /// instances of the struct
    fn prerender(&self, _pass: &mut wgpu::RenderPass<'_>) {}

    fn fragment_targets(renderer: &mut Renderer) -> SmallVec<[Option<wgpu::ColorTargetState>; 4]> {
        Self::Pass::fragment_targets(renderer)
    }

    fn create_bind_group_layouts(device: &wgpu::Device) -> Vec<BindGroupLayout> {
        Self::CONFIG
            .variable_bind_groups
            .into_iter()
            .filter_map(|x| x.map(|y| device.create_bind_group_layout(&y)))
            .collect::<Vec<_>>()
    }
}
pub const fn sum_immediate_sizes<T: MaterialType + ?Sized>() -> u32 {
    use std::mem::size_of;
    (size_of::<T::Immediates>() + size_of::<<T::Pass as RenderPass>::Immediates>()) as u32
}
pub struct MaterialConfig {
    /// TODO: make this variable length? otherwise create_bind_group_layouts is slow
    pub variable_bind_groups: [Option<BindGroupLayoutDescriptor<'static>>; 2],
    pub buffers: &'static [VertexBufferLayout<'static>],
    // pub immediate_size: u32,
    pub shader_config: (ShaderConfig, ShaderConfig),
    pub primitive: PrimitiveState,
    pub multisample: wgpu::MultisampleState,
    pub multiview_mask: Option<NonZeroU32>,
    pub depth_stencil: Option<wgpu::DepthStencilState>,
    // pub color_targets: &[Option<wgpu::ColorTargetState>]
}
impl MaterialConfig {
    const DEFAULT_PRIMITIVE_STATE: PrimitiveState = PrimitiveState {
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: Some(wgpu::Face::Back),
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        unclipped_depth: false,
        polygon_mode: wgpu::PolygonMode::Fill,
        conservative: false,
    };
    const DEFAULT_MULTISAMPLE_STATE: MultisampleState = wgpu::MultisampleState {
        count: 1,
        mask: !0,
        alpha_to_coverage_enabled: false,
    };
    const DEFAULT_DEPTH_STENCIL: wgpu::DepthStencilState = wgpu::DepthStencilState {
        format: crate::resources::texture::Texture::DEPTH_FORMAT,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::LessEqual,
        stencil: wgpu::StencilState {
            front: wgpu::StencilFaceState::IGNORE,
            back: wgpu::StencilFaceState::IGNORE,
            read_mask: 0,
            write_mask: 0,
        },
        bias: wgpu::DepthBiasState {
            constant: 0,
            slope_scale: 0.,
            clamp: 0.,
        },
    };

    pub const fn new(shader_config: (ShaderConfig, ShaderConfig)) -> Self {
        Self {
            variable_bind_groups: [None, None],
            buffers: &[],
            // immediate_size: 0,
            shader_config,
            primitive: Self::DEFAULT_PRIMITIVE_STATE,
            multisample: Self::DEFAULT_MULTISAMPLE_STATE,
            multiview_mask: None,
            depth_stencil: Some(Self::DEFAULT_DEPTH_STENCIL),
            // color_targets: &[]
        }
    }

    pub const fn with_bind_groups(
        mut self,
        variable_bind_groups: [Option<BindGroupLayoutDescriptor<'static>>; 2],
    ) -> Self {
        self.variable_bind_groups = variable_bind_groups;
        self
    }
    pub const fn with_buffers(mut self, buffers: &'static [VertexBufferLayout<'static>]) -> Self {
        self.buffers = buffers;
        self
    }
    // pub const fn with_immediates(mut self, immediate_size: u32) -> Self {
    //     self.immediate_size = immediate_size;
    //     self
    // }
    pub const fn with_primitive_state(mut self, primitive: PrimitiveState) -> Self {
        self.primitive = primitive;
        self
    }
    pub const fn with_multisample(mut self, multisample: wgpu::MultisampleState) -> Self {
        self.multisample = multisample;
        self
    }
    pub const fn with_depth_stencil(
        mut self,
        depth_stencil: Option<wgpu::DepthStencilState>,
    ) -> Self {
        self.depth_stencil = depth_stencil;
        self
    }
    //  pub const fn with_targets(mut self, color_targets: &[Option<wgpu::ColorTargetState>]) -> Self {
    //     self.color_targets = color_targets;
    //     self
    // }
    pub const fn with_multiview_mask(mut self, multiview_mask: Option<NonZeroU32>) -> Self {
        self.multiview_mask = multiview_mask;
        self
    }
}

//TODO: default immeditate size, depth stencil
pub(crate) struct ShaderConfig {
    pub shader_path: &'static str,
    pub entry_point: Option<&'static str>,
    pub compilation_options: PipelineCompilationOptions<'static>,
}
impl ShaderConfig {
    const DEFAULT_COMPILATION_OPTIONS: PipelineCompilationOptions<'static> =
        PipelineCompilationOptions {
            constants: &[],
            zero_initialize_workgroup_memory: false,
        };
    pub const fn new_pair(
        shader_path: &'static str,
        entry_points: (Option<&'static str>, Option<&'static str>),
    ) -> (Self, Self) {
        (
            Self {
                shader_path,
                entry_point: entry_points.0,
                compilation_options: Self::DEFAULT_COMPILATION_OPTIONS,
            },
            Self {
                shader_path,
                entry_point: entry_points.1,
                compilation_options: Self::DEFAULT_COMPILATION_OPTIONS,
            },
        )
    }
    pub const fn new_single(shader_path: &'static str, entry_point: Option<&'static str>) -> Self {
        Self {
            shader_path,
            entry_point,
            compilation_options: Self::DEFAULT_COMPILATION_OPTIONS,
        }
    }
    pub const fn with_compilation_options(
        mut self,
        compilation_options: PipelineCompilationOptions<'static>,
    ) -> Self {
        self.compilation_options = compilation_options;
        self
    }
}

pub struct BlinnPhong {
    pub material: Self::Material,
}
impl BlinnPhong {
    pub type Material = blinn_phong::Material;
    type RawMaterial = blinn_phong::RawMaterial;

    pub fn push(light: crate::camera::light::LightUniform, context: &mut Context) {
        // HERE ...
    }
    /*pub fn new(material: Self::Material, context: &mut Context) -> Self {
        todo!()
    }*/
}
impl MaterialType for BlinnPhong {
    const CONFIG: MaterialConfig = MaterialConfig::new(ShaderConfig::new_pair(
        "core_shaders/blinn_phong",
        (None, None),
    ))
    .with_buffers(&[ModelVertex::DESC]);

    type Pass = GeometryPass;
    type Immediates = ();
    const IMMEDIATE_SIZE: u32 =
        sum_immediate_sizes::<Self>() + std::mem::size_of::<Self::RawMaterial>() as u32;
    // const IMMEDIATE_OFFSET: u32 = GeometryPass::IMMEDIATES_END //+ std::mem::size_of::<Self::Material>() as u32;

    fn prerender(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_immediates(
            GeometryPass::IMMEDIATES_END,
            bytemuck::bytes_of(&self.material.to_raw()),
        )
    }
}
impl BindGroupGenerator for BlinnPhong {
    fn bind_group_layout(context: &mut Context) -> BindGroupLayout {
        context
            .renderer
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    }
    fn resources(context: &mut Context) -> Vec<BindingResource> {
        vec![BindingResource::Buffer(
            context
                .renderer
                .device
                .create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    usage: wgpu::BufferUsages::STORAGE,
                    contents: &[],
                }),
        )]
    }
}

mod blinn_phong {
    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub struct Material {
        ambient: [f32; 3],
        diffuse: [f32; 3],
        specular: [f32; 3],
        shininess: f32,
    }
    impl Material {
        pub(super) fn to_raw(self) -> RawMaterial {
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
}
