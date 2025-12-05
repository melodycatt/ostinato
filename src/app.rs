use std::sync::Arc;
use winit::{application::ApplicationHandler, dpi::{PhysicalSize, Size}, event::{DeviceEvent, DeviceId, WindowEvent}, event_loop::{ActiveEventLoop, EventLoop}, window::Window};
use crate::{AppHandler, Context, HEIGHT, WIDTH};

/// This holds everything and interfaces with wgpu
pub struct App<T: AppHandler+'static> {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    context: Option<Context>,
    app_handler: T
}

impl<T: AppHandler+'static> App<T> {
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            context: None,
            app_handler: T::new(),
            #[cfg(target_arch = "wasm32")]
            proxy,
        }
    }
}

impl<T: AppHandler+'static> ApplicationHandler<Context> for App<T> {
    // emits when it starts i think
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // TODO: ALLOW CUSTOMIZABLE DIMENSIONS
        let window_attributes = Window::default_attributes().with_inner_size(Size::Physical(PhysicalSize { width: WIDTH, height: HEIGHT }));

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
            self.context = Some(pollster::block_on(Context::new(window)).unwrap());
            let state = match &mut self.context {
                Some(canvas) => canvas,
                None => return,
            };
            pollster::block_on(state.init()).expect("x_x :: couldnt init state");
            pollster::block_on(self.app_handler.init(state)).expect("x_x :: couldnt init handler");
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
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: Context) {
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
        //state.mouse_mut().window_event(event_loop, window_id, &event);
        //state.keyboard_mut().window_event(event_loop, window_id, &event);
        //let wgpu = state.graphics();    

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => context.renderer.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                context.update(&mut self.app_handler).expect("update error");
                match context.render(&mut self.app_handler) {
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
                //if state.start().elapsed().as_millis_f32() > 100000.0 { event_loop.exit(); }
            }
            _ => {}
        }
        context.window_event(event_loop, window_id, event);
    }

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

pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app: App<ExampleHandler> = App::new(
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






struct ExampleHandler {
    cube: Option<crate::mesh::Mesh>,
    cube2: Option<crate::mesh::Mesh>,
    camera: Option<crate::camera::Camera>,
    camera_controller: crate::camera::CameraController
}

impl AppHandler for ExampleHandler {
    fn new() -> Self {
        Self {
            cube: None,
            cube2: None,
            camera: None,
            camera_controller: crate::camera::CameraController::new(0.05)
        }
    }
    async fn init(&mut self, context: &mut Context) -> anyhow::Result<()> {
        let cam = crate::camera::Camera::new(crate::camera::CameraConfig {
            eye: (0.0, 5.0, 2.0).into(),
            rotation: <cgmath::Quaternion<f32> as cgmath::Rotation3>::from_axis_angle(cgmath::Vector3::unit_y(), cgmath::Rad(std::f32::consts::PI)),
            fovy: 60.0,
            znear: 0.01,
            zfar: 1000.0,
        }, context.renderer.config.width as f32 / context.renderer.config.height as f32, &context.renderer.device);
        context.renderer.shared_bind_groups.insert("CAMERA", (cam.bind_group.clone(), cam.bind_group_layout.clone()));
        self.camera = Some(cam);
        let _ = anyhow::Context::with_context(crate::resources::load_shader("shaders/static", &mut context.renderer, Some("static_fill"), None).await, || "error when loading shader")?;
        self.cube = Some(crate::mesh::new_cube([0.,0.,0.], [1.,1.,1.], "static_fill", &mut context.renderer));
        let _ = anyhow::Context::with_context(crate::resources::load_shader("bathroom/blue", &mut context.renderer, Some("static_wire"), Some(wgpu::PrimitiveState { polygon_mode: wgpu::PolygonMode::Line, ..Default::default() })).await, || "error when loading shader")?;
        self.cube2 = Some(crate::mesh::new_cube([-0.05,-0.05,-0.05], [1.1,1.1,1.1], "static_wire", &mut context.renderer));

        Ok(())
    }
    fn render(&mut self, context: &mut Context, pass: &mut wgpu::RenderPass<'_>) -> anyhow::Result<(), wgpu::SurfaceError> {
        context.renderer.render_with_camera(pass, &mut self.camera.as_mut().unwrap(), &self.cube.as_ref().unwrap()).expect("AAA");
        context.renderer.render_with_camera(pass, &mut self.camera.as_mut().unwrap(), &self.cube2.as_ref().unwrap()).expect("AAA");
        Ok(())
    }
    fn update(&mut self, context: &mut Context) -> anyhow::Result<()> {
        let camera = self.camera.as_mut().unwrap();
        context.renderer.queue.write_buffer(&camera.buffer, 0, bytemuck::cast_slice(&[camera.uniform]));
        self.camera_controller.update_camera(camera, &context.mouse, &context.keyboard);
        camera.uniform.update_view_proj(camera.config());
        Ok(())
    }
}