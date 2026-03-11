use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BufferDescriptor, CommandEncoder, FragmentState,
    PipelineLayoutDescriptor, PrimitiveState, RenderPipeline, RenderPipelineDescriptor,
    ShaderStages, VertexBufferLayout, VertexState,
};

use crate::Context;
use crate::renderer::InstanceRaw;
use crate::resources::{BufferLayout, ModelVertex, VertexBuffer, load_shader};
use crate::{prelude::Renderer, resources::BindingResource};

pub trait BindGroupGenerator: 'static {
    fn bind_group(
        layout: &BindGroupLayout,
        resources: &[BindingResource],
        renderer: &mut Renderer,
    ) -> BindGroup {
        todo!()
    }
    fn bind_group_layout(renderer: &mut Renderer) -> BindGroupLayout {
        todo!()
    }
    fn resources(renderer: &mut Renderer) -> Vec<BindingResource> {
        todo!()
    }
}
pub trait RenderPass: BindGroupGenerator {
    type Immediates: bytemuck::Pod;
    // type ImmediateTypes: ...;
    fn render_pass<'a>(
        renderer: &mut Renderer,
        encoder: &'a mut CommandEncoder,
    ) -> wgpu::RenderPass<'a> {
        todo!()
    }
}

struct GeometryPass {}
impl RenderPass for GeometryPass {
    type Immediates = InstanceRaw;
}
impl BindGroupGenerator for GeometryPass {}

pub(crate) struct PassData {
    bind_group: BindGroup,
    layout: BindGroupLayout,
    resources: Vec<BindingResource>,
}
impl PassData {
    pub fn from_pass<T: BindGroupGenerator>(renderer: &mut Renderer) -> Self {
        let resources = T::resources(renderer);
        let layout = T::bind_group_layout(renderer);
        Self {
            bind_group: T::bind_group(&layout, &resources, renderer),
            layout: layout,
            resources: resources,
        }
    }
}
pub(crate) struct MaterialData {
    bind_group: BindGroup,
    resources: Vec<BindingResource>,
    pipeline: RenderPipeline,
}
impl MaterialData {
    pub fn from_material<T: MaterialType>(renderer: &mut Renderer) -> Self {
        let resources = T::resources(renderer);
        let layout = T::bind_group_layout(renderer);

        let bind_group = T::bind_group(&layout, &resources, renderer);
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
    type Immediates: bytemuck::Pod;
    type Pass: RenderPass;
    const INDIVIDUAL_BIND_GROUPS: usize;
    /// automatically calculated from the sizes you provide
    /// you should never define this yourself unless youre doing weird memory stuff in the shader?
    /// i guess?
    
    const BUFFERS: &[VertexBufferLayout<'static>];
    const SHADERS: [(&'static str, &'static str); 2];

    fn pipeline(layout: BindGroupLayout, context: &mut Context) -> RenderPipeline;
}
pub struct MaterialConfig {
    variable_bind_groups: [Option<BindGroupLayoutDescriptor<'static>; 2],
    buffers: &'static [VertexBufferLayout<'static>],
    immediate_size: u32,

}

pub struct BlinnPhong {
    pub material: Self::Material,
}
impl BlinnPhong {
    pub type Material = blinn_phong::Material;
    type RawMaterial = blinn_phong::RawMaterial;

    pub fn new(material: Self::Material, context: &mut Context) -> Self {
        todo!()
    }
}
impl MaterialType for BlinnPhong {
    const INDIVIDUAL_BIND_GROUPS: usize = 0;
    fn pipeline(layout: BindGroupLayout, context: &mut Context) -> RenderPipeline {
        let device = context.renderer.device.clone();
        let shader = load_shader("core_shaders/blinn_phong", context).unwrap();
        let bgs = &[
            layout,
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
                            has_dynamic_offset: true,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                }),
        ];
        let layout = Some(&context.renderer.device.create_pipeline_layout(
            &PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: bgs,
                immediate_size: (size_of::<Self::Immediates>()
                    + size_of::<<Self::Pass as RenderPass>::Immediates>())
                    as u32,
            },
        ));
        context
            .renderer
            .device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: None,
                cache: None,
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: crate::resources::texture::Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                layout,
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: Self::BUFFERS,
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: context.renderer.config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState {
                    cull_mode: Some(wgpu::Face::Back),
                    front_face: wgpu::FrontFace::Ccw,
                    ..Default::default()
                },
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
            })
    }

    const BUFFERS: &[VertexBufferLayout<'static>] = &[ModelVertex::desc()];
    type Pass = GeometryPass;
    type Immediates = Self::RawMaterial;
}
impl BindGroupGenerator for BlinnPhong {
    fn bind_group(
        layout: &BindGroupLayout,
        resources: &[BindingResource],
        renderer: &mut Renderer,
    ) -> BindGroup {
        renderer.device.create_bind_group(&BindGroupDescriptor {
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
    fn bind_group_layout(renderer: &mut Renderer) -> BindGroupLayout {
        renderer
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
    fn resources(renderer: &mut Renderer) -> Vec<BindingResource> {
        vec![BindingResource::Buffer(renderer.device.create_buffer_init(
            &BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsages::STORAGE,
                contents: &[],
            },
        ))]
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

use std::marker::PhantomData;
use std::mem::size_of;

pub struct Nil;

pub struct Cons<H, T>(PhantomData<(H, T)>);

pub trait TypeList<Rhs = Nil> {
    type Concat;
    const SIZE: usize;
}
impl<Rhs> TypeList<Rhs> for Nil {
    type Concat = Rhs;
    const SIZE: usize = 0;
}
impl<H, T, Rhs> TypeList<Rhs> for Cons<H, T>
where
    T: TypeList<Rhs>,
{
    type Concat = Cons<H, <T as TypeList<Rhs>>::Concat>;
    const SIZE: usize = size_of::<H>() + T::SIZE;
}

#[macro_export]
macro_rules! type_list {
    () => { Nil };
    ($head:ty $(, $tail:ty)*) => {
        Cons<$head, type_list!($($tail),*)>
    };
}

trait Test {
    const N: usize;
    fn foo() -> [usize; Self::N];
}
