use glam::Mat4;
use std::{any::TypeId, collections::HashMap, ops::Range, sync::Arc, time::Instant};
use wgpu::{
    BindGroup, BufferDescriptor, CommandEncoder, Features, RenderPass, SurfaceError,
    SurfaceTexture, util::DeviceExt,
};
use winit::window::Window;

use crate::{
    Context,
    camera::{Camera, CameraUniform},
    resources::{
        BindingResource, Material, ResourceCollection, ResourceId, Texture, VertexBuffer,
        pipeline::{MaterialData, PassData},
    },
};

pub type EntryLayoutGenerator = fn(u32) -> wgpu::BindGroupLayoutEntry;
/// the rendering context and everything that handles it
pub struct Renderer {
    /// basically the window. wgpu stuff
    pub(crate) surface: wgpu::Surface<'static>,
    pub device: Arc<wgpu::Device>,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub(crate) is_surface_configured: bool,
    pub(crate) depth_texture: Texture,
    pub(crate) window: Arc<Window>,

    /// shader resources for bind groups such as buffers
    /// use the `downcast_ref` and `downcast_mut` methods for these
    pub shader_resources: ResourceCollection<BindingResource>,
    /// NOT SHADERS!!! MATERIALS!!! im a bot
    pub materials: ResourceCollection<Material>,
    /// shared bindgroups so that you dont have to go through the process of remaking them every damn time
    /// not sure if this has a performance difference but i think so?
    pub shared_bindings: ResourceCollection<(BindingResource, EntryLayoutGenerator)>,

    pub material_types: HashMap<TypeId, MaterialData>,
    pub passes: HashMap<TypeId, PassData>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ResolutionUniform {
    res: [f32; 2],
}

impl Renderer {
    /// dont worry :)
    pub(crate) async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::POLYGON_MODE_LINE | Features::IMMEDIATES,
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                required_limits: wgpu::Limits {
                    max_immediate_size: adapter.limits().max_immediate_size,
                    ..Default::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off, // Trace path.
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        window.set_cursor_visible(false);
        window.set_cursor_grab(winit::window::CursorGrabMode::Locked)?;

        let depth_texture =
            Texture::create_depth_texture(&device, (config.width, config.height), "depth_texture");

        let camera_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("shared camera buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            mapped_at_creation: false,
            usage: wgpu::BufferUsages::COPY_DST.union(wgpu::BufferUsages::UNIFORM),
        });
        let mut shared_bindings = ResourceCollection::new();
        // TODO: i guess turn this into a function
        shared_bindings.insert(
            "CAMERA",
            (
                camera_buffer.into(),
                CameraUniform::binding_generator as EntryLayoutGenerator,
            ),
        );
        // TODO: let user set initial cameraconfig (maybe)
        Ok(Self {
            surface,
            device: Arc::new(device),
            queue,
            config,
            is_surface_configured: false,
            depth_texture,
            window,
            shader_resources: ResourceCollection::new(),
            materials: ResourceCollection::new(),
            shared_bindings,
            material_types: HashMap::new(),
            passes: HashMap::new(),
        })
    }

    pub(crate) async fn init(&mut self) -> anyhow::Result<()> {
        let time_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Time Buffer"),
                contents: bytemuck::cast_slice(&[Instant::now().elapsed().as_secs_f32()]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let res = self.window.inner_size().cast::<f32>().into();
        let res_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Resolution Buffer"),
                contents: bytemuck::cast_slice(&[ResolutionUniform { res }]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        self.shader_resources.insert("time", time_buffer);
        self.shader_resources.insert("resolution", res_buffer);

        Ok(())
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.is_surface_configured = true;
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            let t = Texture::create_depth_texture(
                &self.device,
                (self.config.width, self.config.height),
                "depth_texture",
            );
            self.depth_texture = t;
        }
    }

    pub fn command_encoder(&mut self) -> CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            })
    }
    pub fn render_pass<'b>(
        &mut self,
        encoder: &'b mut CommandEncoder,
    ) -> anyhow::Result<(SurfaceTexture, wgpu::RenderPass<'b>), SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;
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
                view: &self.depth_texture.view,
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

    pub fn set_camera(&mut self, camera: &Camera) {
        let buffer = self.get_shared_resource(0).as_inner_buffer();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Vertex Buffer Copy Encoder"),
            });
        encoder.copy_buffer_to_buffer(
            &camera.buffer,
            0,
            buffer,
            0,
            std::mem::size_of::<CameraUniform>() as u64,
        );
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn get_shared_resource(&self, id: impl ResourceId) -> &BindingResource {
        &self.shared_bindings.get(id).unwrap().0
    }
    pub fn get_shared_resource_mut(&mut self, id: impl ResourceId) -> &mut BindingResource {
        &mut self.shared_bindings.get_mut(id).unwrap().0
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Instance {
    pub position: glam::Vec3,
    pub pivot: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}
impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        let model_matrix = Mat4::from_translation(self.position + self.pivot)
            * Mat4::from_quat(self.rotation)
            * Mat4::from_scale(self.scale)
            * Mat4::from_translation(-self.pivot);
        let normal = glam::Mat3::from_mat4(model_matrix)
            .inverse()
            .transpose()
            .to_cols_array_2d();

        InstanceRaw {
            model: model_matrix.to_cols_array_2d(),
            normal0: normal[0],
            _pad0: 0.,
            normal1: normal[1],
            _pad1: 0.,
            normal2: normal[2],
            _pad2: 0.,
        }
    }
}
// NEW!
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
    normal0: [f32; 3],
    _pad0: f32,
    normal1: [f32; 3],
    _pad1: f32,
    normal2: [f32; 3],
    _pad2: f32,
}
impl VertexBuffer for InstanceRaw {
    const STEP_MODE: wgpu::VertexStepMode = wgpu::VertexStepMode::Instance;
    // fn desc(vertex_attrs: &[wgpu::VertexAttribute]) -> wgpu::VertexBufferLayout<'_> {
    //     use std::mem;
    //     wgpu::VertexBufferLayout {
    //         array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
    //         // We need to switch from using a step mode of Vertex to Instance
    //         // This means that our shaders will only change to use the next
    //         // instance when the shader starts processing a new instance
    //         step_mode: wgpu::VertexStepMode::Instance,
    //         attributes: vertex_attrs,
    //     }
    // }
    const ATTRS: &'static [wgpu::VertexAttribute] = &[
        wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
            shader_location: 1,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
            shader_location: 2,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: std::mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
            shader_location: 3,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: std::mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
            shader_location: 4,
            format: wgpu::VertexFormat::Float32x3,
        },
        wgpu::VertexAttribute {
            offset: std::mem::size_of::<[f32; 20]>() as wgpu::BufferAddress,
            shader_location: 5,
            format: wgpu::VertexFormat::Float32x3,
        },
        wgpu::VertexAttribute {
            offset: std::mem::size_of::<[f32; 24]>() as wgpu::BufferAddress,
            shader_location: 6,
            format: wgpu::VertexFormat::Float32x3,
        },
    ];
}

pub trait Renderable {
    /// draw all instances possible
    fn draw(&self, pass: &mut RenderPass, context: &mut Context) -> anyhow::Result<()> {
        self.draw_instances(pass, 0..1, context)
    }
    /// draw a range of instances
    fn draw_instances(
        &self,
        pass: &mut RenderPass,
        instances: Range<u32>,
        context: &mut Context,
    ) -> anyhow::Result<()>;
}
