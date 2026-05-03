use crate::{AppHandler, Context};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, Event, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
};

// TODO remove wasm its annoying and uselesss since we use line polygon mode feature anyway

/// this holds everything and interfaces with wgpu
struct App<T: AppHandler + 'static> {
    context: Option<Context>,
    app_handler: Option<T>,
}

impl<T: AppHandler + 'static> App<T> {
    pub fn new() -> Self {
        Self {
            context: None,
            app_handler: None,
        }
    }
}

impl<T: AppHandler + 'static> ApplicationHandler<Context> for App<T> {
    // emits when it starts i think
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // TODO: ALLOW CUSTOMIZABLE DIMENSIONS
        let window_attributes = T::window_attributes();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut context = pollster::block_on(Context::new(window)).unwrap();
            pollster::block_on(context.init()).expect("AAA");
            self.app_handler = Some(pollster::block_on(T::new(&mut context)).unwrap());

            self.context = Some(context);
        }
    }

    /// USELESS!!
    /// literally only happens if *I* emit an event. so why would i not just run it myself
    /// idk this is just useless
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: Context) {
        self.context = Some(event);
    }

    /// window event...
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let context = match &mut self.context {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                context.renderer.resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                let handler = match &mut self.app_handler {
                    Some(handler) => handler,
                    None => return,
                };
                context.update(handler).expect("update error");
                match context.render(handler) {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = context.renderer.window.inner_size();
                        context.renderer.resize(size.width, size.height);
                    }
                    Err(_) => {
                        println!("idk bru sorry")
                    }
                };
            }
            WindowEvent::Occluded(false) => context.renderer.window.request_redraw(),
            e => {
                println!("{e:?}");
                context
                    .window_event(event_loop, window_id, e)
                    .expect("x_x :: AAAA AAAAAAAA WERE ALL GONNA DIE");
            }
        }
    }

    /// mostlt mouse stuff
    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let state = match &mut self.context {
            Some(canvas) => canvas,
            None => return,
        };
        state.device_event(event_loop, device_id, event);
    }
}

pub fn run<T: AppHandler + 'static>() -> anyhow::Result<()> {
    let event_loop = EventLoop::with_user_event().build()?;
    let mut app: App<T> = App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}
