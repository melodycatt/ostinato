use glam::Mat4;
use std::time::Instant;
use std::{ops::Range, sync::Arc};
use wgpu::util::DeviceExt;
use wgpu::{CommandEncoder, Features, RenderPass, SurfaceError, SurfaceTexture};
use winit::window::Window;

use crate::Context;
use crate::mesh::vertex::VertexBuffer;
use crate::resources::{Texture, load_shader};

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
    pub(crate) scene_texture: Texture,

    pub(crate) delta_instant: Instant,
    /// time between frames, in seconds
    pub delta: f64,
    /// init time
    pub start: Instant,
    pub post_uniform: (wgpu::Buffer, wgpu::BindGroupLayout, wgpu::BindGroup),
    pub(crate) scene_bind_group: (wgpu::BindGroupLayout, wgpu::BindGroup),
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PostUniform {
    time: f32,
    _pad: f32,
    res: [f32; 2],
}

impl Renderer {
    pub fn window(&self) -> &Window {
        &self.window
    }
    /// dont worry :)
    pub(crate) async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
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

        // let surface_caps = surface.get_capabilities(&adapter);
        //
        // let surface_format = surface_caps
        //     .formats
        //     .iter()
        //     .copied()
        //     .find(|f| f.is_srgb())
        //     .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::PostMultiplied,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        window.set_visible(true);
        window.focus_window();
        let depth_texture =
            Texture::create_depth_texture(&device, (config.width, config.height), "depth_texture");

        let scene_texture = Texture::create_render_texture(&device, &config);
        let scene_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post texture bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });
        let scene_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post texture bg"),
            layout: &scene_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&scene_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&scene_texture.sampler),
                },
            ],
        });

        let post_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("post uniform buffer"),
            contents: bytemuck::bytes_of(&PostUniform {
                time: 0.,
                _pad: 0.,
                res: [size.width as f32, size.height as f32],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let post_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post uniform bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::all(),
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let post_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post uniform bg"),
            layout: &post_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: post_buf.as_entire_binding(),
            }],
        });
        // let camera_buffer = device.create_buffer(&BufferDescriptor {
        //     label: Some("shared camera buffer"),
        //     size: std::mem::size_of::<CameraUniform>() as u64,
        //     mapped_at_creation: false,
        //     usage: wgpu::BufferUsages::COPY_DST.union(wgpu::BufferUsages::UNIFORM),
        // });
        // let mut shared_bindings = ResourceCollection::new();
        // // TODO: i guess turn this into a function
        // shared_bindings.insert(
        //     "CAMERA",
        //     (
        //         camera_buffer.into(),
        //         CameraUniform::binding_generator as EntryLayoutGenerator,
        //     ),
        // );
        // TODO: let user set initial cameraconfig (maybe)
        Ok(Self {
            surface,
            device: Arc::new(device),
            queue,
            config,
            is_surface_configured: false,
            depth_texture,
            window,
            scene_texture,
            scene_bind_group: (scene_bgl, scene_bind_group),

            delta: 0.,
            delta_instant: Instant::now(),
            start: Instant::now(),
            post_uniform: (post_buf, post_bgl, post_bg),
        })
    }

    // pub(crate) async fn init(&mut self) -> anyhow::Result<()> {
    //     Ok(())
    // }

    pub fn res(&self) -> [f32; 2] {
        self.window.inner_size().cast::<f32>().into()
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
            let t = Texture::create_render_texture(&self.device, &self.config);
            self.scene_texture = t;
            let scene_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("post texture bg"),
                layout: &self.scene_bind_group.0,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.scene_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.scene_texture.sampler),
                    },
                ],
            });
            self.scene_bind_group.1 = scene_bind_group;
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
    ) -> anyhow::Result<wgpu::RenderPass<'b>, SurfaceError> {
        // let surface_texture = self.surface.get_current_texture()?;
        // let view = surface_texture
        //     .texture
        //     .create_view(&wgpu::TextureViewDescriptor::default());
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.scene_texture.view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.,
                        g: 0.,
                        b: 0.,
                        a: 0.,
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

        Ok(pass)
    }
    pub fn post_pass<'b>(
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
                        a: 0.,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            multiview_mask: None,
            timestamp_writes: None,
        });

        Ok((surface_texture, pass))
    }
    // pub fn set_camera(&mut self, camera: &Camera) {
    //     let buffer = self.get_shared_resource(0).as_inner_buffer();
    //     let mut encoder = self
    //         .device
    //         .create_command_encoder(&wgpu::CommandEncoderDescriptor {
    //             label: Some("Vertex Buffer Copy Encoder"),
    //         });
    //     encoder.copy_buffer_to_buffer(
    //         &camera.buffer,
    //         0,
    //         buffer,
    //         0,
    //         std::mem::size_of::<CameraUniform>() as u64,
    //     );
    //     self.queue.submit(std::iter::once(encoder.finish()));
    // }

    // pub fn get_shared_resource(&self, id: impl ResourceId) -> &BindingResource {
    //     &self.shared_bindings.get(id).unwrap().0
    // }
    // pub fn get_shared_resource_mut(&mut self, id: impl ResourceId) -> &mut BindingResource {
    //     &mut self.shared_bindings.get_mut(id).unwrap().0
    // }
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
    pub fn to_mat4(&self) -> glam::Mat4 {
        // Translate to position, apply pivot, then TRS, then unpivot
        let t = glam::Mat4::from_translation(self.position);
        let p = glam::Mat4::from_translation(self.pivot);
        let ip = glam::Mat4::from_translation(-self.pivot);
        let r = glam::Mat4::from_quat(self.rotation);
        let s = glam::Mat4::from_scale(self.scale);

        t * p * r * s * ip
    }

    pub fn from_mat4(m: glam::Mat4) -> Self {
        let (scale, rotation, translation) = m.to_scale_rotation_translation();
        Self {
            position: translation,
            rotation,
            scale,
            pivot: glam::Vec3::ZERO, // pivot is "baked in" after composition
        }
    }

    /// Applies `other` after `self`
    pub fn apply(&self, other: &Instance) -> Instance {
        let m1 = self.to_mat4();
        let m2 = other.to_mat4();
        Instance::from_mat4(m2 * m1)
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
    const STRIDE: u32 = 7;
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
    fn attrs(location: u32) -> Vec<wgpu::VertexAttribute> {
        use std::mem;
        [
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: location,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                shader_location: location + 1,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                shader_location: location + 2,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                shader_location: location + 3,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                shader_location: location + 4,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: mem::size_of::<[f32; 20]>() as wgpu::BufferAddress,
                shader_location: location + 5,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: mem::size_of::<[f32; 24]>() as wgpu::BufferAddress,
                shader_location: location + 6,
                format: wgpu::VertexFormat::Float32x3,
            },
        ]
        .to_vec()
    }
}

pub trait Renderable {
    /// draw all instances possible
    fn draw(&self, pass: &mut RenderPass, renderer: &mut Renderer) {
        self.draw_instances(pass, 0..1, renderer)
    }
    /// draw a range of instances
    fn draw_instances(&self, pass: &mut RenderPass, instances: Range<u32>, renderer: &mut Renderer);
}

pub fn post_pipeline(
    shader_path: &str,
    immediate_size: u32,
    context: &mut Context,
) -> wgpu::RenderPipeline {
    let device = context.renderer.device.clone();
    let module = load_shader(shader_path, context).unwrap();
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("post pipeline layout"),
        bind_group_layouts: &[
            &context.renderer.post_uniform.1,
            &context.renderer.scene_bind_group.0,
        ],
        immediate_size,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("post pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: None,
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),

        depth_stencil: None,
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
