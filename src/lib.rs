use std::{any::{Any, TypeId}, cell::{Ref, RefCell, RefMut}, collections::HashMap, env, iter, num::NonZeroU32, sync::Arc, time::Instant};

use cgmath::{One, Quaternion};
use hecs::{Component, Entity, World};
use wgpu::{util::DeviceExt, BindGroupLayout, Buffer, PipelineLayout, PresentMode};
use winit::{
    application::ApplicationHandler, dpi::{PhysicalSize, Size}, event::*, event_loop::{ActiveEventLoop, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::Window
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::{camera::{controller::CameraController, Camera, CameraConfig, CameraUniform}, input::{keyboard::KeyboardData, mouse::MouseData}, mesh::Mesh, texture::Texture};

pub mod texture;
pub mod mesh;
pub mod camera;
pub mod input;
pub mod shader;

const WIDTH: u32 = 1000;
const HEIGHT: u32 = 1000;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TimeUniform {
    time: f32,
}

/*pub struct Context {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub is_surface_configured: bool,
    //render_pipeline: wgpu::RenderPipeline,

    pub depth_texture: texture::Texture,

    pub camera: Camera,
    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    //camera_bind_group: wgpu::BindGroup,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub camera_controller: CameraController,

    pub start: Instant,
    /*time_bind_group: wgpu::BindGroup,
    pub time_uniform: TimeUniform,*/
    pub time_buffer: wgpu::Buffer,
    //vertex_buffer: wgpu::Buffer,
    //index_buffer: wgpu::Buffer,
    // NEW!
    //#[allow(dead_code)]
    // diffuse_texture: texture::Texture,
    // diffuse_bind_group: wgpu::BindGroup,
    pub window: Arc<Window>,

    pub mouse: MouseData,
    pub keyboard: KeyboardData
}*/
pub struct State {
    delta: Instant,
    world: World,
    entities: Vec<Entity>,
    /// im not gonna stop you from mutating core resources
    /// but make sure you know what youre doing if you do that
    /// i dont even know what'll happen
    /// 
    /// youre sort of not allowed to remove them though. 
    /// thatll panic the next time theyre accessed, so you can technically swap them out.
    /// 
    /// i would say to avoid removing resources at all because of id exhaustion,
    /// but lets be real. youre not gonna have 4 billion resources.
    /// i dont know how you would have 4 billion resources.
    /// 
    /// RESOURCES ARE NOT MEANT FOR ENTITIES!
    resources: HashMap<ResourceId, RefCell<Box<dyn Any>>>
}

/// if youre mad about the nested resource types
/// stay mad.
/// it would be so much worse if we had them all in one enum
/// then it would either be unclear what each is without docs (bad)
/// or we prefix every variant with relevant info (eg. ResourceId::WgpuSurface) (also bad)
/// actually maybe that last one aint THAT bad but TOO LATE LOL
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum ResourceId {
    Wgpu,
    Camera,
    DepthTexture,
    Window,
    Start,
    Mouse,
    Keyboard,
    Custom(u32)
}
/*#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum WgpuResource {
    Surface,
    Device,
    Queue,
    Config,
    IsSurfaceConfigured
}*/
pub struct WgpuResource {
    pub surface: wgpu::Surface<'static>,
    pub device: Arc<wgpu::Device>,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub is_surface_configured: bool,
}
// this will need to change if i want multiple cameras
/*#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum CameraResource {
    Camera,
    Uniform,
    Buffer,
    BindGroupLayout,
    // get rid of this, just make it a system
    // put the controller logic into a state method that can be called by the user
    // (unused by the engine)
    Controller,
}*/

impl State {
    /// makes a new state! self.init() will be called immediately after this
    async fn new(window: Arc<Window>) -> anyhow::Result<State> {
        let size = window.inner_size();
        let start = Instant::now();

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
                    required_features: wgpu::Features::empty(),
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

        let time_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Time Buffer"),
                contents: bytemuck::cast_slice(&[start.elapsed().as_secs_f32()]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }    
        );    

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

        let camera_config = CameraConfig {
            eye: (0.0, 0.0, 2.0).into(),
            rotation: Quaternion::one(),
            fovy: 60.0,
            znear: 0.01,
            zfar: 1000.0,
        };

        let camera = Camera::new(camera_config, config.width as f32 / config.height as f32, &device);

        let depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let mut resources: HashMap<ResourceId, RefCell<Box<dyn Any + 'static>>> = HashMap::new();
        resources.insert(ResourceId::Wgpu, RefCell::new(Box::new(WgpuResource {
            surface,
            device: Arc::new(device),
            queue,
            config,
            is_surface_configured: false
        })));
        resources.insert(ResourceId::Camera, RefCell::new(Box::new(camera)));
        resources.insert(ResourceId::DepthTexture, RefCell::new(Box::new(depth_texture)));
        resources.insert(ResourceId::Start, RefCell::new(Box::new(start)));
        resources.insert(ResourceId::Window, RefCell::new(Box::new(window)));
        resources.insert(ResourceId::Mouse, RefCell::new(Box::new(MouseData::new())));
        resources.insert(ResourceId::Keyboard, RefCell::new(Box::new(KeyboardData::new())));
        resources.insert(ResourceId::Custom(0), RefCell::new(Box::new(CameraController::new(0.05))));
        resources.insert(ResourceId::Custom(1), RefCell::new(Box::new(time_buffer)));

        Ok(Self {
            delta: Instant::now(),
            world: World::new(),
            entities: Vec::new(),
            resources
        })
    }

    pub fn create_resource<T: 'static>(&mut self, id: ResourceId, value: T) {
        self.resources.insert(id, RefCell::new(Box::new(value)));
    }
    pub fn downcast_resource<T: 'static>(&self, id: &ResourceId) -> Ref<'_, T> {
        //self.resources.get(id).expect(&format!("x_x :: tried to access nonexistent resource with id {:?}", id))
        //    .downcast_ref().expect(&format!("x_x :: tried to downcast resource with id {:?} to incorrect type: {}", id, std::any::type_name::<T>()))
        let cell = self.resources.get(id).expect(&format!("x_x :: tried to access nonexistent resource with id {:?}", id));
        Ref::map(cell.borrow(), |b| b.downcast_ref::<T>().expect(&format!("x_x :: tried to downcast resource with id {:?} to incorrect type: {}", id, std::any::type_name::<T>())))
    }
    pub fn downcast_resource_mut<T: 'static>(&self, id: &ResourceId) -> RefMut<'_, T> {
        //self.resources.get_mut(id).expect(&format!("x_x :: tried to access nonexistent resource with id {:?}", id))
        //    .downcast_mut().expect(&format!("x_x :: tried to downcast resource with id {:?} to incorrect type: {}", id, std::any::type_name::<T>()))
        let cell = self.resources.get(id).expect(&format!("x_x :: tried to access nonexistent resource with id {:?}", id));
        RefMut::map(cell.borrow_mut(), |b| b.downcast_mut::<T>().expect(&format!("x_x :: tried to downcast resource with id {:?} to incorrect type: {}", id, std::any::type_name::<T>())))
    }
    /// you probably dont want this
    pub fn get_resource(&self, id: &ResourceId) -> Option<&RefCell<Box<dyn Any + 'static>>> {
        self.resources.get(id)
    }
    /// you probably dont want this
    pub fn get_resource_mut(&mut self, id: &ResourceId) -> Option<&mut RefCell<Box<dyn Any + 'static>>> {
        self.resources.get_mut(id)
    }
    
    pub fn init(&mut self) {
        //let shader = self.device.create_shader_module(wgpu::include_wgsl!("resources/shaders/static.wgsl"));
        //
        /*let time_uniform = TimeUniform {
            time: sstart.elapsed().as_secs_f32()
        };*/
        //let resource = self.downcast_resource::<wgpu::Buffer>(&ResourceId::Custom(1));
        let device = self.wgpu().device.clone();

        let time_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },    
                    count: None,
                }    
            ],    
            label: Some("time_bind_group_layout"),
        });    
        let time_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &time_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.downcast_resource::<wgpu::Buffer>(&ResourceId::Custom(1)).as_entire_binding(),
                }    
            ],    
            label: Some("time_bind_group"),
        });
        let mesh1 = mesh::new_cube([0., 0., 0.], &[time_bind_group], &[&time_bind_group_layout], env::current_dir().expect("couldnt get current dir?").join("src/resources/shaders/static.wgsl"), self);
        self.entities.push(self.world.spawn((mesh1,)));
        let mesh2 = mesh::new_cube([1.5, 1.5, 1.5], &[], &[], env::current_dir().expect("couldnt get current dir?").join("src/resources/shaders/mesh.wgsl"), self);
        self.entities.push(self.world.spawn((mesh2,)));
    }

    /*pub fn add_render_object(&mut self, obj: impl RenderObject + 'static) {
        self.render_objects.push(Box::new(obj));
    }*/

    pub fn render_pipeline_layout(&self, bindings: &[&BindGroupLayout]) -> PipelineLayout {
        let device = self.wgpu().device.clone();
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: [&[&self.camera().bind_group_layout], bindings].concat().as_slice(),
            push_constant_ranges: &[],
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            let mut wgpu = self.wgpu_mut();
            wgpu.is_surface_configured = true;
            wgpu.config.width = width;
            wgpu.config.height = height;
            wgpu.surface.configure(&wgpu.device, &wgpu.config);
            let t = texture::Texture::create_depth_texture(&wgpu.device, &wgpu.config, "depth_texture");
            drop(wgpu);
            self.create_resource(ResourceId::DepthTexture, t);
        }
    }

    fn handle_key(&mut self, event_loop: &ActiveEventLoop, key: KeyCode, pressed: bool) {
        match (key, pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        }
    }

    fn update(&mut self) {
        println!("delta in ms: {}", self.delta.elapsed().as_millis());
        {
            let mut camera = self.camera_mut();
            let time_buffer = self.downcast_resource::<Buffer>(&ResourceId::Custom(1));

            //let time_buffer = self.downcast_resource_mut::<Buffer>(&ResourceId::Custom(1));
            let wgpu = self.wgpu_mut();
            self.downcast_resource_mut::<CameraController>(&ResourceId::Custom(0)).update_camera(&mut camera, &self.mouse(), &self.keyboard());
            let camera_config = camera.config();
            camera.uniform.update_view_proj(camera_config);
            //self.time_uniform.time = self.start.elapsed().as_secs_f32();
            wgpu.queue.write_buffer(&time_buffer, 0, bytemuck::cast_slice(&[self.start().elapsed().as_secs_f32()]));
            wgpu.queue.write_buffer(&camera.buffer, 0, bytemuck::cast_slice(&[camera.uniform]));
            self.mouse_mut().update();
            self.keyboard_mut().update();
        }
        self.delta = Instant::now();
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.window().request_redraw();

        let mut wgpu = self.wgpu_mut();

        // We can't render unless the surface is configured
        if !wgpu.is_surface_configured {
            return Ok(());
        }
        
        let output = wgpu.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = wgpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                    view: &self.depth_texture().view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            for (_, mesh) in self.world.query::<&Mesh>().iter() {
                pass.set_pipeline(&mesh.shader.render_pipeline);
                for i in 0..mesh.shader.bind_groups.len() {
                    pass.set_bind_group(i as u32, &mesh.shader.bind_groups[i], &[]);
                }
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
            }
            //render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            /*for obj in 0..self.render_objects.len() {
                self.render_objects[obj].render(&mut render_pass);
            }*/
            
            //render_pass.set_bind_group(1, &self.time_bind_group, &[]);
        }

        wgpu.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn wgpu(&self) -> Ref<'_, WgpuResource> {
        self.downcast_resource(&ResourceId::Wgpu)
    }
    pub fn camera(&self) -> Ref<'_, Camera> {
        self.downcast_resource(&ResourceId::Camera)
    }
    pub fn depth_texture(&self) -> Ref<'_, Texture> {
        self.downcast_resource(&ResourceId::DepthTexture)
    }
    pub fn window(&self) -> Ref<'_, Arc<Window>> {
        self.downcast_resource(&ResourceId::Window)
    }
    pub fn start(&self) -> Ref<'_, Instant> {
        self.downcast_resource(&ResourceId::Start)
    }
    pub fn mouse(&self) -> Ref<'_, MouseData> {
        self.downcast_resource(&ResourceId::Mouse)
    }
    pub fn keyboard(&self) -> Ref<'_, KeyboardData> {
        self.downcast_resource(&ResourceId::Keyboard)
    }
    pub fn wgpu_mut(&self) -> RefMut<'_, WgpuResource> {
        self.downcast_resource_mut(&ResourceId::Wgpu)
    }
    pub fn camera_mut(&self) -> RefMut<'_, Camera> {
        self.downcast_resource_mut(&ResourceId::Camera)
    }
    pub fn depth_texture_mut(&self) -> RefMut<'_, Texture> {
        self.downcast_resource_mut(&ResourceId::DepthTexture)
    }
    pub fn window_mut(&self) -> RefMut<'_, Arc<Window>> {
        self.downcast_resource_mut(&ResourceId::Window)
    }
    pub fn start_mut(&self) -> RefMut<'_, Instant> {
        self.downcast_resource_mut(&ResourceId::Start)
    }
    pub fn mouse_mut(&self) -> RefMut<'_, MouseData> {
        self.downcast_resource_mut(&ResourceId::Mouse)
    }
    pub fn keyboard_mut(&self) -> RefMut<'_, KeyboardData> {
        self.downcast_resource_mut(&ResourceId::Keyboard)
    }
}

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
}

impl App {
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
        }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes().with_inner_size(Size::Physical(PhysicalSize { width: WIDTH, height: HEIGHT }));

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            // If we are not on web we can use pollster to
            // await the
            self.state = Some(pollster::block_on(State::new(window)).unwrap());
            let state = match &mut self.state {
                Some(canvas) => canvas,
                None => return,
            };
            state.init();
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(proxy
                        .send_event(
                            State::new(window)
                                .await
                                .expect("Unable to create canvas!!!")
                        )
                        .is_ok())
                });
            }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        #[cfg(target_arch = "wasm32")]
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };
        state.mouse_mut().window_event(event_loop, window_id, &event);
        state.keyboard_mut().window_event(event_loop, window_id, &event);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = state.window().inner_size();
                        state.resize(size.width, size.height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {}", e);
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => match (button, state.is_pressed()) {
                (MouseButton::Left, true) => {}
                (MouseButton::Left, false) => {}
                _ => {}
            },
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            _ => {}
        }
    }

    fn device_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            device_id: DeviceId,
            event: DeviceEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };
        state.mouse_mut().device_event(event_loop, device_id, &event);
    }
}

fn assert_send_sync<T: Send + Sync + 'static>() {
    println!("send and sync? {:?}", TypeId::of::<T>())
}
fn assert_component<T: Component>() {
    println!("component? {:?}", TypeId::of::<T>())
}

pub fn run() -> anyhow::Result<()> {
    assert_send_sync::<wgpu::Buffer>();
    assert_component::<wgpu::Buffer>();
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new(
        #[cfg(target_arch = "wasm32")]
        &event_loop,
    );
    event_loop.run_app(&mut app)?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();

    Ok(())
}

/*macro_rules! bind_group_layout {
    ($device:ident, $($binding:expr, $visibility:expr, $ty:expr, $count:expr),*, $label:expr) => {
        $device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[$
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: ,
                }
            ],
            label: Some($label),
        });
    };
}*/