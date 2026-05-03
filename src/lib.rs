mod app;
pub mod camera;
pub mod input;
pub mod mesh;
pub mod renderer;
pub mod resources;
use anyhow::Result;
pub use app::*;
// pub use glam;
// pub use wgpu;
// TODO remove this
// pub use bytemuck;
use std::{
    iter,
    sync::Arc,
    time::{Duration, Instant},
};
use wgpu::{RenderPass, RenderPipeline};

// TODO: make this customizable
pub const WIDTH: u32 = 1000;
pub const HEIGHT: u32 = 1000;

use winit::{
    dpi::{PhysicalSize, Size},
    event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

use crate::{
    input::{keyboard::KeyboardData, mouse::MouseData},
    prelude::post_pipeline,
    renderer::Renderer,
};

// TODO: didnt know where to put this.
// it would be nice to have some kind of render queue so that internally we can reorder draw calls
// to, say, reduce how often we set bind groups
//
/// context; all the information required to run the app
pub struct Context {
    /// handles alll the rendering shit
    pub renderer: Renderer,

    /// stored resource indices for speed
    /// will often be pre-set seeing as i know the order of my own default resources
    /// sorry, its magic numbers - i lowkey think theyre fun sometimes
    pub(crate) resources_path: Option<String>,

    /// mouse input information
    pub mouse: MouseData,
    /// keyboard input information
    pub keyboard: KeyboardData,

    nothing_shader: Option<RenderPipeline>,

    // last_frame: Instant,
    #[cfg(feature = "rapier3d")]
    /// rapier3d physics context
    pub rapier: rapier::RapierContext<(), ()>,
}

const FPS: Duration = Duration::from_millis(1000 / 60);

impl Context {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        Ok(Self {
            nothing_shader: None,
            renderer: Renderer::new(window).await?,
            // in order:
            // time buffer
            resources_path: None,

            mouse: MouseData::new(true),
            keyboard: KeyboardData::new(),
            // last_frame: Instant::now(),
            #[cfg(feature = "rapier3d")]
            rapier: rapier::RapierContext::new((), ()),
        })
    }

    pub fn set_resource_directory(&mut self, path: String) {
        self.resources_path = Some(path);
    }

    async fn init(&mut self) -> anyhow::Result<()> {
        self.renderer.start = Instant::now();
        self.renderer.delta_instant = Instant::now();
        // self.renderer.init().await?;
        Ok(())
    }

    /// once before every fram
    fn update<T: AppHandler>(&mut self, handler: &mut T) -> anyhow::Result<()> {
        self.renderer.delta = self.renderer.delta_instant.elapsed().as_secs_f64();
        // println!("{}", self.renderer.delta);
        self.renderer.delta_instant = Instant::now();
        self.renderer.queue.write_buffer(
            &self.renderer.post_uniform.0,
            0,
            bytemuck::bytes_of(&self.renderer.start.elapsed().as_secs_f32()),
        );
        //
        // let time_buffer = self
        //     .renderer
        //     .get_shared_resource(self.resource_indices[0])
        //     .as_inner_buffer();
        // self.renderer.queue.write_buffer(
        //     time_buffer,
        //     0,
        //     bytemuck::cast_slice(&[self.start.elapsed().as_secs_f32()]),
        // );

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
        let mut pass = self.renderer.render_pass(&mut encoder)?;

        // let the user render
        handler.render(self, &mut pass)?;

        // pass borrows encoder which conflicts with encoder. should just use a scope but im silly 😝
        drop(pass);
        self.renderer.queue.submit(iter::once(encoder.finish()));
        let mut encoder = self.renderer.command_encoder();
        let (tex, mut pass) = self.renderer.post_pass(&mut encoder)?;
        pass.set_bind_group(0, Some(&self.renderer.post_uniform.2), &[]);
        pass.set_bind_group(1, Some(&self.renderer.scene_bind_group.1), &[]);
        handler.post_process(self, &mut pass)?;
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
    ) -> anyhow::Result<()> {
        self.mouse.window_event(event_loop, window_id, &event);
        self.keyboard.window_event(event_loop, window_id, &event);

        if let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } = event
        {
            input::mouse::lock_and_hide_cursor(false, self);
        }
        Ok(())
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        self.mouse.device_event(event_loop, device_id, &event);
    }

    pub fn pass_post_processing(
        &mut self,
        pass: &mut RenderPass,
    ) -> anyhow::Result<(), wgpu::SurfaceError> {
        if self.nothing_shader.is_none() {
            self.nothing_shader = Some(post_pipeline(
                "core_shaders/post_processing/nothing.wgsl",
                0,
                self,
            ));
        }
        let shader = self.nothing_shader.as_ref().unwrap();
        pass.set_pipeline(shader);
        pass.draw(0..3, 0..1);
        Ok(())
    }

    /*pub fn set_camera(&mut self, camera: &Camera) {
        self.renderer.shared_bind_groups.
    }*/
}

pub trait AppHandler: Sized {
    #[allow(async_fn_in_trait)]
    async fn new(context: &mut Context) -> Result<Self>;

    /// called once every frame. you are given one `RenderPass`
    /// if you want another, contribute to the library and make `Context` a trait so you can make custom event loops
    // TODO not make this a surfacerror
    fn render(
        &mut self,
        context: &mut Context,
        pass: &mut RenderPass<'_>,
    ) -> Result<(), wgpu::SurfaceError>;
    /// called once before `render`
    fn update(&mut self, context: &mut Context) -> Result<()>;
    // called when the app is started, since you dont have `context` properly initialised until then
    //fn init(&mut self, context: &mut Context) -> impl std::future::Future<Output = anyhow::Result<()>>;
    /// id imagine this is called right after rendering aga
    fn post_process(
        &mut self,
        context: &mut Context,
        pass: &mut RenderPass<'_>,
    ) -> Result<(), wgpu::SurfaceError>;

    fn window_attributes() -> WindowAttributes {
        WindowAttributes::default().with_inner_size(Size::Physical(PhysicalSize {
            width: WIDTH,
            height: HEIGHT,
        }))
    }
}

pub mod prelude {
    pub use crate::{
        AppHandler, Context, camera,
        mesh::{self, Mesh, vertex},
        renderer::{Instance, Renderable, Renderer, post_pipeline},
        resources::{load_model, load_pipeline},
    };
    // TODO: dont do this.
    pub use anyhow::Result;
    pub use bytemuck;
    pub use glam;
    pub use wgpu;
    pub use winit;
}

#[cfg(feature = "rapier3d")]
pub mod rapier {
    use glam::Vec3;
    use rapier3d::prelude::*;

    pub struct RapierContext<H: PhysicsHooks, E: EventHandler> {
        pub gravity: Vec3,
        pub integration_parameters: IntegrationParameters,
        pub physics_pipeline: PhysicsPipeline,
        pub island_manager: IslandManager,
        pub broad_phase: BroadPhaseBvh,
        pub narrow_phase: NarrowPhase,
        pub ccd_solver: CCDSolver,
        pub impulse_joint_set: ImpulseJointSet,
        pub multibody_joint_set: MultibodyJointSet,
        pub rigid_body_set: RigidBodySet,
        pub collider_set: ColliderSet,
        pub physics_hooks: H,
        pub event_handler: E,
    }
    impl<H: PhysicsHooks, E: EventHandler> RapierContext<H, E> {
        pub fn new(physics_hooks: H, event_handler: E) -> Self {
            Self {
                rigid_body_set: RigidBodySet::new(),
                collider_set: ColliderSet::new(),
                gravity: Vec3::new(0., -9.81, 0.),
                integration_parameters: IntegrationParameters::default(),
                physics_pipeline: PhysicsPipeline::new(),
                island_manager: IslandManager::new(),
                broad_phase: DefaultBroadPhase::new(),
                narrow_phase: NarrowPhase::new(),
                impulse_joint_set: ImpulseJointSet::new(),
                multibody_joint_set: MultibodyJointSet::new(),
                ccd_solver: CCDSolver::new(),
                physics_hooks,
                event_handler,
            }
        }
        pub fn step(&mut self) {
            self.physics_pipeline.step(
                self.gravity,
                &self.integration_parameters,
                &mut self.island_manager,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                &mut self.rigid_body_set,
                &mut self.collider_set,
                &mut self.impulse_joint_set,
                &mut self.multibody_joint_set,
                &mut self.ccd_solver,
                &self.physics_hooks,
                &self.event_handler,
            );
        }
    }
}
