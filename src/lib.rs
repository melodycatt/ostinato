pub mod camera;
pub mod input;
pub mod resources;
pub mod renderer;
pub mod particle;
mod app;
pub use app::*;
pub use wgpu;
pub use glam;
// TODO remove this
pub use bytemuck;
use wgpu::RenderPass;

use std::{iter, sync::Arc, time::Instant};

pub use derive_resource::Resource;

// TODO: make this customizable
pub const WIDTH: u32 = 1000;
pub const HEIGHT: u32 = 1000;

use winit::{event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent}, event_loop::ActiveEventLoop, keyboard::{KeyCode, PhysicalKey}, window::Window};

use crate::{input::{keyboard::KeyboardData, mouse::MouseData}, renderer::Renderer};
//TODO: clean up imports


/// context; all the information required to run the app
pub struct Context {
    /// handles alll the rendering shit
    pub renderer: Renderer,
    
    /// stored resource indices for speed
    /// will often be pre-set seeing as i know the order of my own default resources
    /// sorry, its magic numbers - i lowkey think theyre fun sometimes
    resource_indices: [usize; 1],
    pub(crate)resources_path: Option<String>,

    delta_instant: Instant,
    /// time between frames, in seconds
    pub delta: f64,
    /// init time
    pub start: Instant,

    /// mouse input information
    pub mouse: MouseData,
    /// keyboard input information
    pub keyboard: KeyboardData,
}

impl Context {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        Ok(Self { 
            renderer: Renderer::new(window).await?,
            // in order:
            // time buffer
            resource_indices: [0],
            start: Instant::now(),
            delta_instant: Instant::now(),
            delta: 0.,
            resources_path: None,

            mouse: MouseData::new(),
            keyboard: KeyboardData::new()
        })
    }

    pub fn set_resource_directory(&mut self, path: String) {
        self.resources_path = Some(path);
    }

    async fn init(&mut self) -> anyhow::Result<()> {
        self.start = Instant::now();
        self.delta_instant = Instant::now();
        self.renderer.init().await?;
        Ok(())
    }

    /// once before every fram
    fn update<T: AppHandler>(&mut self, handler: &mut T) -> anyhow::Result<()> {
        self.delta = self.delta_instant.elapsed().as_secs_f64();
        self.delta_instant = Instant::now();

        let time_buffer = self.renderer.shader_resources.downcast_mut::<wgpu::Buffer>(self.resource_indices[0]).unwrap();
        self.renderer.queue.write_buffer(&time_buffer, 0, bytemuck::cast_slice(&[self.start.elapsed().as_secs_f32()]));

        handler.update(self)?;
        
        self.mouse.update();
        self.keyboard.update();

        Ok(())
    }

    /// renders frames!
    fn render<T: AppHandler>(&mut self, handler: &mut T) -> anyhow::Result<(), wgpu::SurfaceError> {
        // must be done before rendering and shit
        self.renderer.window.request_redraw();
        // instead of throwing an error we just pass this frame and wait for it to be true
        if !self.renderer.is_surface_configured {
            return Ok(());
        }
        let mut encoder = self.renderer.command_encoder();
        let (tex, mut pass) = self.renderer.render_pass(&mut encoder)?;

        // let the user render
        handler.render(self, &mut pass)?;

        // pass borrows encoder which conflicts with encoder. should just use a scope but im silly 😝
        drop(pass);
        self.renderer.queue.submit(iter::once(encoder.finish()));
        tex.present();
        Ok(())
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.mouse.window_event(event_loop, window_id, &event);
        self.keyboard.window_event(event_loop, window_id, &event);

        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                event_loop.exit()
            },
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        self.mouse.device_event(event_loop, device_id, &event);
    }
}


pub trait AppHandler: Sized {
    #[allow(async_fn_in_trait)]
    async fn new(context: &mut Context) -> anyhow::Result<Self>;

    /// called once every frame. you are given one `RenderPass`
    /// if you want another, contribute to the library and make `Context` a trait so you can make custom event loops
    // TODO not make this a surfacerror
    fn render(&mut self, context: &mut Context, pass: &mut RenderPass<'_>) -> anyhow::Result<(), wgpu::SurfaceError>;
    /// called once before `render`
    fn update(&mut self, context: &mut Context) -> anyhow::Result<()>;
    // called when the app is started, since you dont have `context` properly initialised until then
    //fn init(&mut self, context: &mut Context) -> impl std::future::Future<Output = anyhow::Result<()>>;
} 




pub mod prelude {
    pub use glam;
    pub use wgpu;
    pub use crate::{
        renderer::Renderer,
        Context,
        AppHandler,
        camera,
        resources::{
            Mesh,
            load_material,
            load_model
        }
    };
}