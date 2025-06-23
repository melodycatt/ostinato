#![feature(duration_millis_float)]
use std::{any::{Any, TypeId}, cell::{Ref, RefCell, RefMut}, collections::HashMap, hash::Hash, iter,  sync::Arc, time::Instant};

pub use derive_resource::Resource;

use anyhow::anyhow;
use as_any::AsAny;
use cgmath::{One, Quaternion};
use hecs::{Component, Entity, World};
use nohash_hasher::BuildNoHashHasher;
use wgpu::{util::DeviceExt, BindGroupLayout, Buffer, PipelineLayout};
use winit::{
    application::ApplicationHandler, dpi::{PhysicalSize, Size}, event::*, event_loop::{ActiveEventLoop, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::Window
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::{camera::{controller::CameraController, Camera, CameraConfig}, input::{keyboard::KeyboardData, mouse::MouseData}, mesh::{Material, Mesh}, resources::{load_shader, load_texture, Resource}, texture::Texture};

pub mod texture;
pub mod mesh;
pub mod camera;
pub mod input;
pub mod resources;

const WIDTH: u32 = 1000;
const HEIGHT: u32 = 1000;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TimeUniform {
    time: f32,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ResolutionUniform {
    res: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable, Resource)]
struct RaymarchUniform {
    time: f32, // time in secs (?)
    delta: f32, // delta in ms
    res: [f32; 2], // screen resolution
}

/// the state.
/// this contains everything you'll need (probably)
/// dont access the `RefCell` fields directly, use their corresponding methods
pub struct State {
    graphics: RefCell<Box<dyn Resource>>,
    start: RefCell<Box<dyn Resource>>,
    delta: RefCell<Box<dyn Resource>>,
    mouse: RefCell<Box<dyn Resource>>,
    keyboard: RefCell<Box<dyn Resource>>,
    camera: RefCell<Box<dyn Resource>>,

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
    // i dont know why this is pub. ill leave it there for now but i dont think this should be pub
    pub resources: HashMap<u64, RefCell<Box<dyn Resource>>, BuildNoHashHasher<u64>>,
    //resource_labels: HashMap<&'static str, ResourceId>,
}

/// `ResourceId`s are for accessing resources.
/// for the core resources (id's without associated `u64`s), access is faster
/// as it does not require a hashmap lookup
const MATERIAL_TAG: u64 = 0x1000_0000_0000_0000;
const BUFFER_TAG:   u64 = 0x2000_0000_0000_0000;
const CUSTOM_TAG:   u64 = 0x3000_0000_0000_0000;
const TAG_MASK:     u64 = 0xF000_0000_0000_0000;
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum ResourceId {
    Invalid,

    GraphicsContext,
    Camera,
    Start,
    Delta,
    Mouse,
    Keyboard,
    Material(u64),
    Buffer(u64),
    Custom(u64),
}
impl ResourceId {
    fn hash_str(s: &str) -> u64 {
        Self::hash(s.to_owned())
    }
    fn hash(s: String) -> u64 {
        use std::hash::{Hasher, DefaultHasher};
        //println!("hashing {s}");
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        return hasher.finish()
    }

    pub fn key(&self) -> Option<u64> {
        match self {
            ResourceId::Material(id) => Some(MATERIAL_TAG | (id & !TAG_MASK)),
            ResourceId::Buffer(id)   => Some(BUFFER_TAG   | (id & !TAG_MASK)),
            ResourceId::Custom(id)   => Some(CUSTOM_TAG   | (id & !TAG_MASK)),
            _ => None,
        }
    }

    pub fn from_key(key: u64) -> ResourceId {
        match key & TAG_MASK {
            MATERIAL_TAG => ResourceId::Material(key & !TAG_MASK),
            BUFFER_TAG   => ResourceId::Buffer(key & !TAG_MASK),
            CUSTOM_TAG   => ResourceId::Custom(key & !TAG_MASK),
            _            => ResourceId::Invalid, // or panic
        }
    }
}
impl From<String> for ResourceId {
    fn from(s: String) -> Self {
        match s.as_str() {
            "core::wgpu" | "core::graphics" | "core::gctx" => ResourceId::GraphicsContext,
            "core::camera" => ResourceId::Camera,
            "core::start" => ResourceId::Start,
            "core::delta" => ResourceId::Start,
            "core::mouse" | "core::mouse_data" => ResourceId::Mouse,
            "core::keyboard" | "core::keyboard_data" => ResourceId::Keyboard,
            _ if s.starts_with("core::") => ResourceId::Invalid,
            _ if s.starts_with("buffer::") => ResourceId::Buffer(Self::hash(s)),
            _ if s.starts_with("material::") => { println!("{s}"); ResourceId::Material(Self::hash(s)) },
            _ => ResourceId::Custom(Self::hash(s)),
        }
    }
}
impl From<&str> for ResourceId {
    fn from(s: &str) -> Self {
        match s {
            "core::wgpu" | "core::graphics" | "core::gctx" => ResourceId::GraphicsContext,
            "core::camera" => ResourceId::Camera,
            "core::start" => ResourceId::Start,
            "core::delta" => ResourceId::Start,
            "core::mouse" | "core::mouse_data" => ResourceId::Mouse,
            "core::keyboard" | "core::keyboard_data" => ResourceId::Keyboard,
            _ if s.starts_with("core::") => ResourceId::Invalid,
            _ if s.starts_with("buffer::") => ResourceId::Buffer(Self::hash_str(s)),
            _ if s.starts_with("material::") => { println!("{s}"); ResourceId::Material(Self::hash_str(s)) },
            _ => ResourceId::Custom(Self::hash_str(s)),
        }
    }
}

#[derive(Debug, Resource)]
pub struct GraphicsContext {
    pub surface: wgpu::Surface<'static>,
    pub device: Arc<wgpu::Device>,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub is_surface_configured: bool,
    pub depth_texture: Texture,
    pub window: Arc<Window>
}

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

        // TODO: let user set initial cameraconfig (maybe)
        let camera_config = CameraConfig {
            eye: (0.0, 0.0, 2.0).into(),
            rotation: Quaternion::one(),
            fovy: 60.0,
            znear: 0.01,
            zfar: 1000.0,
        };

        let camera = Camera::new(camera_config, config.width as f32 / config.height as f32, &device);

        let depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let resources: HashMap<u64, RefCell<Box<dyn Resource + 'static>>, BuildNoHashHasher<u64>> = HashMap::with_hasher(BuildNoHashHasher::<u64>::new());
        let graphics = GraphicsContext {
            surface,
            device: Arc::new(device),
            queue,
            config,
            is_surface_configured: false,
            depth_texture,
            window: window
        };
        //resources.insert(ResourceId::Camera, RefCell::new(Box::new(camera)));
        // resources.insert(ResourceId::Start, RefCell::new(Box::new(start)));
        // resources.insert(ResourceId::Mouse, RefCell::new(Box::new(MouseData::new())));
        // resources.insert(ResourceId::Keyboard, RefCell::new(Box::new(KeyboardData::new())));

        Ok(Self {
            world: World::new(),
            entities: Vec::new(),
            resources,
            
            delta: RefCell::new(Box::new(Instant::now()) as Box<dyn Resource>),
            start: RefCell::new(Box::new(start) as Box<dyn Resource>),
            camera: RefCell::new(Box::new(camera) as Box<dyn Resource>),
            graphics: RefCell::new(Box::new(graphics) as Box<dyn Resource>),
            mouse: RefCell::new(Box::new(MouseData::new()) as Box<dyn Resource>),
            keyboard: RefCell::new(Box::new(KeyboardData::new()) as Box<dyn Resource>),
        })
    }

    /// overwrites existing values
    /// 
    /// if the resource is intended to be used in a bind group on a .omi material, 
    /// T must not have a custom implementation of `binding()` that does not return an error.
    /// this function doesn't check if this is the case, but if you load the material it will panic
    pub fn create_resource<T: Resource>(&mut self, id: ResourceId, value: T) {
        println!("{id:?}");
        self.resources.insert(id.key().expect("x_x :: tried to create invalid resource key"), RefCell::new(Box::new(value)));
    }

    /// borrows the refcell, upcasts its insides to a `dyn Any`, downcasts it to T
    /// panics if the resource doesnt exist, or is not a T
    pub fn downcast_resource<T: Resource + 'static>(&self, id: &ResourceId) -> Ref<'_, T> {
        // get cell
        let cell = self.get_resource(id).expect(&format!("x_x :: tried to downcast nonexistent resource with id: {:?}", id));
        // we map because you cant get a `Ref` to something inside an already borrowed `Ref`
        // this lets you do that
        std::cell::Ref::map(cell, |b| {
            // cast to any first, because it doesnt know how to dispatch for some reason
            let any = b.as_ref() as &dyn Any;
            // downcast!
            any.downcast_ref::<T>().expect(&format!("x_x :: tried to downcast resource with id {:?} to wrong type: {:?}", id, std::any::type_name::<T>()))
        })
    }

    /// borrows the refcell, upcasts its insides to a `dyn Any`, downcasts it to T
    /// panics if the resource does not exist, or is not a T
    pub fn downcast_resource_mut<T: Resource + 'static>(&self, id: &ResourceId) -> RefMut<'_, T> {
        let cell = self.get_resource_mut(id).expect(&format!("x_x :: tried to downcast_mut nonexistent resource with id: {:?}", id));
        std::cell::RefMut::map(cell, |b| {
            let any = b.as_mut() as &mut dyn Any;
            any.downcast_mut::<T>().expect(&format!("x_x :: tried to downcast_mut resource with id {:?} to wrong type: {:?}", id, std::any::type_name::<T>()))
        })
    }

    /// borrows the refcell, upcasts its insides to a `dyn Any`, downcasts it to T
    /// returns an error if the resource doesnt exist, or is not a T
    pub fn try_downcast_resource<T: Resource + 'static>(&self, id: &ResourceId) -> anyhow::Result<Ref<'_, T>> {
        let cell = self
            .get_resource(id)?;

        Ref::filter_map(cell, |boxed| boxed.as_any().downcast_ref::<T>())
            .map_err(|_| anyhow!(
                "x_x :: tried to downcast resource with id {:?} to incorrect type: {}",
                id,
                std::any::type_name::<T>()
            ))
    }

    /// borrows the refcell, upcasts its insides to a `dyn Any`, downcasts it to T
    /// returns an error if the resource doesnt exist, or is not a T
    pub fn try_downcast_resource_mut<T: Resource + 'static>(&self, id: &ResourceId) ->  anyhow::Result<RefMut<'_, T>> {
        let cell = self
            .get_resource_mut(id)?;

        RefMut::filter_map(cell, |boxed| boxed.as_any_mut().downcast_mut::<T>())
            .map_err(|_| anyhow!(
                "x_x :: tried to downcast (mut) resource with id {:?} to incorrect type: {}",
                id,
                std::any::type_name::<T>()
            ))
    }

    /// you probably dont want this
    pub fn get_resource(&self, id: &ResourceId) -> anyhow::Result<Ref<'_, Box<dyn Resource + 'static>>> {
        match id {
            ResourceId::Camera => Ok(self.camera.try_borrow()?),
            ResourceId::Keyboard => Ok(self.keyboard.try_borrow()?),
            ResourceId::GraphicsContext => Ok(self.graphics.try_borrow()?),
            ResourceId::Mouse => Ok(self.mouse.try_borrow()?),
            ResourceId::Start => Ok(self.start.try_borrow()?),
            ResourceId::Delta => Ok(self.delta.try_borrow()?),
            _ => {
                let k = id.key().ok_or(anyhow!("x_x :: tried to get resource with no key: {id:?} (this should never happen)"))?;
                //println!("{:#?}", self.resources.get(id).map(|f| f.try_borrow_mut()).ok_or(anyhow!("x_x :: tried to get nonexistent resource with id: {id:?}"))?);
                Ok(self.resources.get(&k).map(|f| f.try_borrow())
                        .ok_or(anyhow!("x_x :: tried to get nonexistent resource with id: {id:?}"))??)
            },
            _ => panic!("x_x :: tried to get invalid resource key")
        }
    }
    /// you probably dont want this
    pub fn get_resource_mut(&self, id: &ResourceId) -> anyhow::Result<RefMut<'_, Box<dyn Resource + 'static>>> {
        match id {
            ResourceId::Camera => Ok(self.camera.try_borrow_mut()?),
            ResourceId::Keyboard => Ok(self.keyboard.try_borrow_mut()?),
            ResourceId::GraphicsContext => Ok(self.graphics.try_borrow_mut()?),
            ResourceId::Mouse => Ok(self.mouse.try_borrow_mut()?),
            ResourceId::Start => Ok(self.start.try_borrow_mut()?),
            ResourceId::Delta => Ok(self.delta.try_borrow_mut()?),
            _ => {
                let k = id.key().ok_or(anyhow!("x_x :: tried to get resource with no key: {id:?} (this should never happen)"))?;
                //println!("{:#?}", self.resources.get(id).map(|f| f.try_borrow_mut()).ok_or(anyhow!("x_x :: tried to get nonexistent resource with id: {id:?}"))?);
                Ok(self.resources.get(&k).map(|f| f.try_borrow_mut())
                        .ok_or(anyhow!("x_x :: tried to get nonexistent resource with id: {id:?}"))??)
            },
            _ => panic!("x_x :: tried to get invalid resource key")
        }
    }
    
    /// called immediately after new, if you need functions that require `self`
    async fn init(&mut self) -> anyhow::Result<()> {
        let wgpu = self.graphics();
        let device = wgpu.device.clone();
        let queue = &wgpu.queue;
        let window = wgpu.window.clone();
        let start = self.start();

        let time_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Time Buffer"),
                contents: bytemuck::cast_slice(&[start.elapsed().as_secs_f32()]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }    
        );    
        let res =  window.inner_size().cast::<f32>().into();
        let res_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Time Buffer"),
                contents: bytemuck::cast_slice(&[ResolutionUniform { res }]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }    
        );    

        let t = load_texture("crow.png", &device, &queue).await?;

        let march = RaymarchUniform {
            time: start.elapsed().as_secs_f32(),
            delta: start.elapsed().as_millis_f32(),
            res: window.inner_size().cast::<f32>().into(),
        };
        let march_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("March Buffer"),
                contents: bytemuck::cast_slice(&[march]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }    
        );    
        drop(wgpu);
        drop(start);

        self.create_resource("camera_controller".into(), CameraController::new(0.05));
        self.create_resource("buffer::time".into(), time_buffer);
        self.create_resource("buffer::res".into(), res_buffer);
        self.create_resource("buffer::march".into(), march_buffer);
        self.create_resource("custom::march".into(), march);
        self.create_resource("texture::crow".into(), t.view);
        self.create_resource("sampler::crow".into(), t.sampler);

        //let m = pollster::block_on(resources::load_model("shaders/image", self)).expect("A!");

        //for i in m.meshes {
            //println!("{i:#?}");
            //self.entities.push(self.world.spawn((i,)));
        //}

        pollster::block_on(load_shader("march/march", self)).expect("IDIOT");
        //self.create_resource(shader, value);
        //self.entities.push(self.world.spawn((mesh1,)));
        //let mesh2 = mesh::new_cube([1.5, 1.5, 1.5], "shaders/mesh", self).expect("IDIOT");
        //self.entities.push(self.world.spawn((mesh2,)));

        // this is shadertoy stuff, keeping it just in case
        //let plane = Shadertoy::new(env::current_dir().expect("couldnt get current dir?").join("src/resources/shaders/sphere.wgsl"), &[st_bind_group], &[&st_bind_group_layout], self);
        //self.entities.push(self.world.spawn(plane));

        Ok(())
    }

    // a relic of an ancient and naive time... you can stay
    /*pub fn add_render_object(&mut self, obj: impl RenderObject + 'static) {
        self.render_objects.push(Box::new(obj));
    }*/

    /// do we even need this function? (now that we have OMI)
    fn _render_pipeline_layout(&self, bindings: &[&BindGroupLayout]) -> PipelineLayout {
        let device = self.graphics().device.clone();
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: [&[self.camera().bind_group_layout.as_ref()], bindings].concat().as_slice(),
            push_constant_ranges: &[],
        })
    }

    /// idk man ask learn-wgpu
    /// all i know is we reconfigure the surface on resie and create a new depth texture to fit
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            let mut wgpu = self.graphics_mut();
            wgpu.is_surface_configured = true;
            wgpu.config.width = width;
            wgpu.config.height = height;
            wgpu.surface.configure(&wgpu.device, &wgpu.config);
            let t = texture::Texture::create_depth_texture(&wgpu.device, &wgpu.config, "depth_texture");
            drop(wgpu);
            self.graphics_mut().depth_texture = t;
        }
    }

    /// ??
    fn handle_key(&mut self, event_loop: &ActiveEventLoop, key: KeyCode, pressed: bool) {
        match (key, pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        }
    }

    /// you know whats up
    /// called immediately before render()
    fn update(&mut self) {
        //println!("fps: {}", 1000.0 / self.delta().elapsed().as_millis_f64());
        // no real reason to have a block anymore, but it could be useful and its harmless
        {
            let mut camera = self.camera_mut();
            let time_buffer = self.downcast_resource::<Buffer>(&"buffer::time".into());

            let wgpu = self.graphics_mut();
            self.downcast_resource_mut::<CameraController>(&"camera_controller".into()).update_camera(&mut camera, &self.mouse(), &self.keyboard());
            let camera_config = camera.config();
            camera.uniform.update_view_proj(camera_config);
            wgpu.queue.write_buffer(&time_buffer, 0, bytemuck::cast_slice(&[self.start().elapsed().as_secs_f32()]));
            let mut march = self.downcast_resource_mut::<RaymarchUniform>(&"custom::march".into());
            let march_buffer = self.downcast_resource(&"buffer::march".into());
            march.delta = self.delta().elapsed().as_millis_f32();
            march.time = self.start().elapsed().as_secs_f32();
            wgpu.queue.write_buffer(&march_buffer, 0, bytemuck::cast_slice(&[*march]));
            wgpu.queue.write_buffer(&time_buffer, 0, bytemuck::cast_slice(&[self.start().elapsed().as_secs_f32()]));
            wgpu.queue.write_buffer(&camera.buffer, 0, bytemuck::cast_slice(&[camera.uniform]));
            self.mouse_mut().update();
            self.keyboard_mut().update();
        }
        *self.delta_mut() = Instant::now();
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        
        let wgpu = self.graphics();
        wgpu.window.request_redraw() ;

        // we cant render unless the surface is configured
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

        // for shadertoy stuff, ill keep this for a while maybe
        // at least until i think of something for general rendering
        /*for (_, toy) in self.world.query::<&Shadertoy>().iter() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &toy.draw_texture.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,/*Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture().view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),*/
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&toy.shader.render_pipeline);
            for i in 0..toy.shader.bind_groups.len() {
                pass.set_bind_group(i as u32, &toy.shader.bind_groups[i], &[]);
            }
            pass.draw(0..6, 0..1);
        }*/ 
    
        // learn-wgpu uses a block -> i use a block
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
                    view: &wgpu.depth_texture.view,
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
                let m = mesh.material(&self);
                pass.set_pipeline(&m.render_pipeline);
                for i in 0..m.bind_groups.len() {
                    pass.set_bind_group(i as u32, &m.bind_groups[i], &[]);
                }
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
            }

            let s = self.downcast_resource::<Material>(&"material::march/march".into());
            pass.set_pipeline(&s.render_pipeline);
            for i in 0..s.bind_groups.len() {
                pass.set_bind_group(i as u32, &s.bind_groups[i], &[]);
            }
            pass.draw(0..3, 0..1);
        }

        wgpu.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    // this feels horrible, and slow, and everything but its fineeeee
    pub fn graphics(&self) -> Ref<'_, GraphicsContext> {
        self.downcast_resource(&ResourceId::GraphicsContext)
    }
    pub fn camera(&self) -> Ref<'_, Camera> {
        self.downcast_resource(&ResourceId::Camera)
    }
    pub fn start(&self) -> Ref<'_, Instant> {
        self.downcast_resource(&ResourceId::Start)
    }
    pub fn delta(&self) -> Ref<'_, Instant> {
        self.downcast_resource(&ResourceId::Delta)
    }
    pub fn mouse(&self) -> Ref<'_, MouseData> {
        self.downcast_resource(&ResourceId::Mouse)
    }
    pub fn keyboard(&self) -> Ref<'_, KeyboardData> {
        self.downcast_resource(&ResourceId::Keyboard)
    }
    pub fn graphics_mut(&self) -> RefMut<'_, GraphicsContext> {
        self.downcast_resource_mut(&ResourceId::GraphicsContext)
    }
    pub fn camera_mut(&self) -> RefMut<'_, Camera> {
        self.downcast_resource_mut(&ResourceId::Camera)
    }
    pub fn start_mut(&self) -> RefMut<'_, Instant> {
        self.downcast_resource_mut(&ResourceId::Start)
    }
    pub fn delta_mut(&self) -> RefMut<'_, Instant> {
        self.downcast_resource_mut(&ResourceId::Delta)
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
            pollster::block_on(state.init()).expect("x_x :: couldnt init state");
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
        //let wgpu = state.graphics();    

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = state.graphics().window.inner_size();
                        state.resize(size.width, size.height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {}", e);
                    }
                };
                //if state.start().elapsed().as_millis_f32() > 100000.0 { event_loop.exit(); }
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





/*struct Shadertoy {
    draw_texture: Texture,
    shader: Material,
}
impl Shadertoy {
    fn new<P: AsRef<Path>>(shader: P, bind_groups: &[BindGroup], layouts: &[&BindGroupLayout], state: &mut State) -> (Self, Mesh) {
        let device = state.wgpu().device.clone();
        let size = wgpu::Extent3d { // 2.
            width: 200,
            height: 200,
            depth_or_array_layers: 1,
        };
        let device_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadertoy texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let draw_texture = Texture {
            view: device_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            texture: device_texture,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                    address_mode_u: wgpu::AddressMode::ClampToEdge,
                    address_mode_v: wgpu::AddressMode::ClampToEdge,
                    address_mode_w: wgpu::AddressMode::ClampToEdge,
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Nearest,
                    mipmap_filter: wgpu::FilterMode::Nearest,
                    ..Default::default()
                }
            )
        };
        let f = fs::read_to_string(shader).expect("shader non existent in creating cube");
        let descriptor = wgpu::ShaderModuleDescriptor {
            label: Some("mesh shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&f)),
        };
    // include_wgsl!()
        let shader = device.create_shader_module(descriptor);
        //println!("{:?}", layouts);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("balala Render Pipeline Layout"),
            bind_group_layouts: layouts,
            push_constant_ranges: &[],
        });

        let mesh = Self::get_mesh([0., 0., 0.,], &draw_texture, state);

        //println!("MAKING SHADERTOY SHADER NOT MESH SHAdER");
        let shader = Material::new_no_stencil(shader, bind_groups.to_vec(), pipeline_layout, &[], &[Some(Material::screen_target(wgpu::TextureFormat::Rgba8Unorm))], &device);//&[Some(shader::Shader::screen_target(wgpu::TextureFormat::Rgba8Unorm))], &device);
        (Self {
            shader,
            draw_texture
        }, mesh)
    }
    fn get_mesh(position: [f32; 3], draw_texture: &Texture, state: &mut State) -> Mesh {
        let verts = &[
            TextureVertex {
                position: [0. + position[0], 0. + position[1], -10. + position[2]],
                tex_coords: [0., 0.]
            },
            TextureVertex {
                position: [1. + position[0], 0. + position[1], -10. + position[2]],
                tex_coords: [1., 0.]
            },
            TextureVertex {
                position: [0. + position[0], 1. + position[1], -10. + position[2]],
                tex_coords: [0., 1.]
            },
            TextureVertex {
                position: [1. + position[0], 1. + position[1], -10. + position[2]],
                tex_coords: [1., 1.]
            },
        ];
        let indices = &[
            0, 2, 1,
            2, 3, 1
        ];

        let device = state.wgpu().device.clone();

        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // This should match the filterable field of the
                    // corresponding Texture entry above.
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });
        
        let diffuse_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&draw_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&draw_texture.sampler),
                    }
                ],
                label: Some("diffuse_bind_group"),
            }
        );

        Mesh::construct(verts, indices, &[diffuse_bind_group], &[&texture_bind_group_layout], env::current_dir().expect("couldnt get current dir?").join("src/resources/shaders/image.wgsl"),state)
        // Mesh::construct(verts, indices, &[], &[], env::current_dir().expect("couldnt get current dir?").join("src/resources/shaders/static_tex.wgsl"), state)
    }
}*/