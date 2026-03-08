use crate::{AppHandler, Context, HEIGHT, WIDTH};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalSize, Size},
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};

// TODO remove wasm its annoying and uselesss since we use line polygon mode feature anyway

/// this holds everything and interfaces with wgpu
struct App<T: AppHandler + 'static> {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    context: Option<Context>,
    app_handler: Option<T>,
}

impl<T: AppHandler + 'static> App<T> {
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            context: None,
            app_handler: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
        }
    }
}

impl<T: AppHandler + 'static> ApplicationHandler<Context> for App<T> {
    // emits when it starts i think
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // TODO: ALLOW CUSTOMIZABLE DIMENSIONS
        let window_attributes =
            Window::default_attributes().with_inner_size(Size::Physical(PhysicalSize {
                width: WIDTH,
                height: HEIGHT,
            }));

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
            let mut context = pollster::block_on(Context::new(window)).unwrap();
            pollster::block_on(context.init()).expect("AAA");
            self.app_handler = Some(pollster::block_on(T::new(&mut context)).unwrap());
            self.context = Some(context);
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        proxy
                            .send_event(
                                State::new(window)
                                    .await
                                    .expect("Unable to create canvas!!!")
                            )
                            .is_ok()
                    )
                });
            }
        }
    }

    /// USELESS!!
    /// literally only happens if *I* emit an event. so why would i not just run it myself
    /// idk this is just useless
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: Context) {
        #[cfg(target_arch = "wasm32")]
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
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
            WindowEvent::Resized(size) => context.renderer.resize(size.width, size.height),
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
                    Err(e) => {
                        log::error!("Unable to render {}", e);
                    }
                };
            }
            _ => {
                context
                    .window_event(event_loop, window_id, event)
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app: App<T> = App::new(
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
