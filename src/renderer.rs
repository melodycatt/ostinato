use std::{sync::Arc, time::Instant};
use wgpu::{Buffer, CommandEncoder, SurfaceError, SurfaceTexture, util::DeviceExt};
use winit::window::Window;

use crate::{camera::Camera, resources::{Resource, ResourceCollection, Material, Mesh, Texture}};

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
    pub shader_resources: ResourceCollection<Box<dyn Resource>>,
    /// NOT SHADERS!!! MATERIALS!!! im a bot
    pub materials: ResourceCollection<Material>,
    /// shared bindgroups so that you dont have to go through the process of remaking them every damn time
    /// not sure if this has a performance difference but i think so?
    pub shared_bind_groups: ResourceCollection<(Arc<wgpu::BindGroup>, Arc<wgpu::BindGroupLayout>)>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ResolutionUniform {
    res: [f32; 2],
}

impl Renderer {
    /// dont worry :)
    pub(crate) async fn new(window: Arc<Window>) -> anyhow::Result<Renderer> {
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
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::POLYGON_MODE_LINE,
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    memory_hints: Default::default(),
                    trace: wgpu::Trace::Off, // Trace path
                },
            )
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

        let depth_texture = Texture::create_depth_texture(&device, (config.width, config.height), "depth_texture");

        // TODO: let user set initial cameraconfig (maybe)
        Ok(Self {
            surface,
            device: Arc::new(device),
            queue,
            config,
            is_surface_configured: false,
            depth_texture,
            window: window,
            shader_resources: ResourceCollection::new(),
            materials: ResourceCollection::new(),
            shared_bind_groups: ResourceCollection::new(),
        })
    }

    pub(crate) async fn init(&mut self) -> anyhow::Result<()> {
        let time_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Time Buffer"),
                contents: bytemuck::cast_slice(&[Instant::now().elapsed().as_secs_f32()]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }    
        );    
        let res = self.window.inner_size().cast::<f32>().into();
        let res_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Resolution Buffer"),
                contents: bytemuck::cast_slice(&[ResolutionUniform { res }]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }    
        );

        let time = Box::new(time_buffer);
        self.shader_resources.insert("time", time);
        self.shader_resources.insert("resolution", Box::new(res_buffer));

        Ok(())
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.is_surface_configured = true;
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            let t = Texture::create_depth_texture(&self.device, (self.config.width, self.config.height), "depth_texture");
            self.depth_texture = t;
        }
    }

    pub fn command_encoder(&mut self) -> CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            })
    }
    pub fn render_pass<'a>(&mut self, encoder: &'a mut CommandEncoder) -> anyhow::Result<(SurfaceTexture, wgpu::RenderPass<'a>), SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
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
            timestamp_writes: None,
        });

        Ok((surface_texture, pass))
    }

    pub fn render_with_camera(&mut self, pass: &mut wgpu::RenderPass, camera: &mut Camera, mesh: &Mesh)-> anyhow::Result<()> {
        self.shared_bind_groups.insert("CAMERA", (camera.bind_group.clone(), camera.bind_group_layout.clone()));

        let m: &Material = self.materials.get(mesh.material)?;
        pass.set_pipeline(&m.render_pipeline);

        for i in 0..m.shared_bind_groups.len() {
            let b = self.shared_bind_groups.get(m.shared_bind_groups[i])?.clone();
            pass.set_bind_group(i as u32, Some(&*b.0), &[]);
        }
        for i in 0..m.bind_groups.len() {
            let b = &m.bind_groups[i];
            pass.set_bind_group((i + m.shared_bind_groups.len()) as u32, Some(b), &[]);
        }
        pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
        Ok(())
    }
    pub fn render_with_camera_instanced(&mut self, pass: &mut wgpu::RenderPass, camera: &mut Camera, mesh: &Mesh, instance_buffer: &Buffer, instances: std::ops::Range<u32>)-> anyhow::Result<()> {
        self.shared_bind_groups.insert("CAMERA", (camera.bind_group.clone(), camera.bind_group_layout.clone()));

        let m: &Material = self.materials.get(mesh.material)?;
        pass.set_pipeline(&m.render_pipeline);

        for i in 0..m.shared_bind_groups.len() {
            let b = self.shared_bind_groups.get(m.shared_bind_groups[i])?.clone();
            pass.set_bind_group(i as u32, Some(&*b.0), &[]);
        }
        for i in 0..m.bind_groups.len() {
            let b = &m.bind_groups[i];
            pass.set_bind_group((i + m.shared_bind_groups.len()) as u32, Some(b), &[]);
        }
        pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.slice(..));
        pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..mesh.indices.len() as u32, 0, instances);
        Ok(())
    }
}


pub struct Instance {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}
impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position).to_cols_array_2d(),
        }
    }
}
// NEW!
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
}
impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in the shader.
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials, we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5, not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
