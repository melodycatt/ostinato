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

    #[cfg(feature = "rapier3d")]
    /// rapier3d physics context
    pub rapier: rapier::RapierContext<(),()>,

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

            mouse: MouseData::new(true),
            keyboard: KeyboardData::new(),

            #[cfg(feature = "rapier3d")]
            rapier: rapier::RapierContext::new((), ())
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
    ) -> anyhow::Result<()> {
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
                self.renderer.window.set_cursor_visible(true);
                self.renderer.window.set_cursor_grab(winit::window::CursorGrabMode::None)?;
            },
            _ => {}
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
    pub use winit;
    pub use crate::{
        renderer::{Renderer, Instance},
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

#[cfg(feature = "rapier3d")]
pub mod rapier {
    use rapier3d::prelude::*;
    use glam::Vec3;
    
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
        pub event_handler: E
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
