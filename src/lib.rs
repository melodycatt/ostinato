pub mod texture;
pub mod mesh;
pub mod camera;
pub mod input;
pub mod resources;
pub mod renderer;
mod app;
pub use app::*;
use wgpu::RenderPass;

use std::{iter, sync::Arc, time::Instant};

pub use derive_resource::Resource;

pub const WIDTH: u32 = 1000;
pub const HEIGHT: u32 = 1000;

use winit::{event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent}, event_loop::ActiveEventLoop, keyboard::{KeyCode, PhysicalKey}, window::Window};

use crate::{input::{keyboard::KeyboardData, mouse::MouseData}, renderer::Renderer};


pub struct Context {
    pub renderer: Renderer,
    pub resource_indices: [usize; 1],
    pub start: Instant,

    pub mouse: MouseData,
    pub keyboard: KeyboardData
}

impl Context {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        Ok(Self { 
            renderer: Renderer::new(window).await?,
            resource_indices: [0],
            start: Instant::now(),

            mouse: MouseData::new(),
            keyboard: KeyboardData::new()
        })
    }

    pub async fn init(&mut self) -> anyhow::Result<()> {
        self.start = Instant::now();
        self.renderer.init().await?;
        self.resource_indices[0] = 0;
        Ok(())
    }

    pub fn update<T: AppHandler>(&mut self, handler: &mut T) -> anyhow::Result<()> {
        let time_buffer = self.renderer.shader_resources.downcast_mut::<wgpu::Buffer>(self.resource_indices[0]).unwrap();
        self.renderer.queue.write_buffer(&time_buffer, 0, bytemuck::cast_slice(&[self.start.elapsed().as_secs_f32()]));
        handler.update(self)?;
        
        self.mouse.update();
        self.keyboard.update();

        Ok(())
    }

    pub fn render<T: AppHandler>(&mut self, handler: &mut T) -> anyhow::Result<(), wgpu::SurfaceError> {
        self.renderer.window.request_redraw();
        if !self.renderer.is_surface_configured {
            return Ok(());
        }
        let mut encoder = self.renderer.command_encoder();
        let (tex, mut pass) = self.renderer.render_pass(&mut encoder)?;

        handler.render(self, &mut pass)?;

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
        //let wgpu = state.graphics();    

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


pub trait AppHandler {
    fn new() -> Self;

    fn render(&mut self, context: &mut Context, pass: &mut RenderPass<'_>) -> anyhow::Result<(), wgpu::SurfaceError>;
    fn update(&mut self, context: &mut Context) -> anyhow::Result<()>;
    fn init(&mut self, context: &mut Context) -> impl std::future::Future<Output = anyhow::Result<()>>;
} 