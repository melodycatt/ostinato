use std::{
    f32::consts::{PI, SQRT_2},
    fmt::Debug,
};

use ostinato::{
    AppHandler, Context,
    camera::light::LightUniform,
    mesh::{
        Model, ObjModel, StorageMesh, new_cube,
        vertex::{ModelVertex, SimpleVertex, StepInstance, VertexBuffer},
    },
    prelude::*,
    resources::{
        Texture,
        blinn_phong::{Material, light_binding},
        load_pipeline, load_texture,
    },
};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, Buffer, BufferUsages, PrimitiveState,
    RenderPipeline,
    util::{BufferInitDescriptor, DeviceExt},
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize, Size},
    platform::macos::WindowAttributesExtMacOS,
    window::WindowAttributes,
};

fn main() {
    ostinato::run::<ExampleHandler>().unwrap();
}

/// example handler. not for outside use.
/// full namespace paths so it doesnt clutter this file
/// so much for above LOLW LOLW LOLW LOWL LOLW LOLW LOLW
#[allow(dead_code)]
pub struct ExampleHandler {
    camera: ostinato::camera::Camera,
    camera_controller: ostinato::camera::CameraController,
    cube: Mesh<ModelVertex>,
    wireframe: StorageMesh,
    clickbait: Clickbait,
    lights: Vec<LightUniform>,
    light_binding: (Buffer, BindGroup),

    pipelines: [RenderPipeline; 5],

    skull: ObjModel,
}
use std::path::PathBuf;

fn resources_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap();

    exe.parent() // MacOS
        .and_then(|p| p.parent()) // Contents
        .map(|p| p.join("Resources/res"))
        .unwrap()
}
impl AppHandler for ExampleHandler {
    async fn new(context: &mut Context) -> anyhow::Result<Self> {
        context.set_resource_directory(resources_dir().to_string_lossy().to_string());
        //j tjhis
        let pipelines = [
            // BLINNPHONG
            load_pipeline("core_shaders/blinn_phong", context, None).await?,
            // WIREFRAME
            load_pipeline(
                "core_shaders/wireframe",
                context,
                Some(PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    cull_mode: None,
                    ..Default::default()
                }),
            )
            .await?,
            // CLICKBAIT
            load_pipeline(
                "core_shaders/clickbait",
                context,
                Some(PrimitiveState {
                    cull_mode: None,
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                }),
            )
            .await?,
            // DITHER
            ostinato::renderer::post_pipeline(
                "core_shaders/post_processing/white_dither.wgsl",
                0,
                context,
            ),
            //
            // OBJ
            load_pipeline("core_shaders/obj", context, None).await?,
        ];

        let mut skull = load_model("skull/human_skull", context).await?;
        skull.meshes.pop();
        let mut mat = Material {
            ambient: [0.2; 3],
            diffuse: [1.; 3],
            specular: [1.; 3],
            shininess: 25.,
        };
        skull.meshes[0].material = mat;
        skull.meshes[2].material = mat;
        mat.shininess = 49.;
        skull.meshes[1].material = mat;
        let transform = Instance {
            position: glam::Vec3::new(-0.5, -0.5, -0.5),
            pivot: glam::Vec3::new(0.5, 0.5, 0.5),
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::new(0.05, 0.05, 0.05),
        };
        skull.transform = transform;

        let renderer = &mut context.renderer;

        let win = renderer.window();
        // win.set_visible(false);
        win.set_cursor_hittest(false).unwrap();

        // let pos = PhysicalPosition {
        //     x: size.width - 500,
        //     y: size.height - 500,
        // };
        // win.current_monitor().unwrap().size()
        // let size = win.current_monitor().unwrap().size();
        // let pos = PhysicalPosition { x: 0, y: 0 };
        // win.set_outer_position(pos);
        // win.request_inner_size(size);
        win.set_maximized(true);
        // win.request_inner_size(PhysicalSize {
        //     width: 600,
        //     height: 600,
        // });

        let lights = vec![LightUniform::new([-3., 5., 6.5], [1., 1., 1.], 30.)];
        let light_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Lgiht buf"),
                contents: bytemuck::cast_slice(&lights),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            });
        let light_bg = light_binding(&renderer.device, &light_buffer);

        // renderer.shader_resources.insert("lights", light_buffer);
        let material = Material {
            diffuse: [0., 1., 0.],
            ambient: [0., 0.1, 0.],
            specular: [0., 1., 0.],
            shininess: 64.,
        };

        let camera = ostinato::camera::Camera::new(
            ostinato::camera::CameraConfig {
                eye: (0.0, -0.2, 6.0).into(),
                rotation: glam::Quat::from_rotation_y(0.) * glam::Quat::from_rotation_x(0.),
                fovy: 32.2,
                znear: 0.01,
                zfar: 1000.0,
            },
            renderer.config.width as f32 / renderer.config.height as f32,
            &renderer.device,
        );

        let cube = new_cube(
            Instance {
                position: glam::Vec3::new(-0.5, -0.5, -0.5),
                pivot: glam::Vec3::new(0.5, 0.5, 0.5),
                rotation: glam::Quat::IDENTITY,
                scale: glam::Vec3::new(1., 1., 1.),
            },
            material,
            &mut context.renderer,
        );
        let wireframe = StorageMesh::from_mesh(cube.clone(), &context.renderer.device)?;

        let clickbait = Clickbait::from_mesh(&cube, context).await?;

        //wireframe.transform.scale = glam::Vec3::new(1.2, 1.2, 1.2);
        //wireframe.transform.rotation = glam::Quat::from_rotation_y(PI);

        Ok(Self {
            // cube: None,
            // cube2: None,
            camera,
            cube,
            wireframe,
            clickbait,
            lights,
            camera_controller: ostinato::camera::CameraController::new(0.15, 1.),
            light_binding: (light_buffer, light_bg),
            pipelines,
            skull: ObjModel::from_model(
                skull,
                &[
                    [
                        "skull/Skull_Jaw_BaseColor.png",
                        "skull/Skull_Jaw_Roughness.png",
                    ],
                    ["skull/Teeth_BaseColor.png", "skull/Teeth_Roughness.png"],
                    [
                        "skull/Skull_Top_BaseColor.png",
                        "skull/Skull_Top_Roughness.png",
                    ],
                ],
                context,
            )
            .await?,
        })
    }
    fn render(
        &mut self,
        context: &mut Context,
        pass: &mut wgpu::RenderPass<'_>,
    ) -> anyhow::Result<(), wgpu::SurfaceError> {
        let size = context.renderer.window().inner_size();
        pass.set_viewport(
            size.width as f32 - 500.,
            size.height as f32 - 500.,
            500.,
            500.,
            0.,
            1.,
        );
        pass.set_bind_group(0, Some(&self.camera.bind_group), &[]);

        pass.set_pipeline(&self.pipelines[4]);
        pass.set_bind_group(1, Some(&self.light_binding.1), &[]);
        // pass.set_bind_group(2, Some(&self.jaw_bg), &[]);
        // self.skull.meshes[0].draw(pass, &mut context.renderer);
        // for i in 0..3 {
        //     pass.set_bind_group(2, Some(&self.skull_bgs[i]), &[]);
        //     self.skull.meshes[i].draw(pass, &mut context.renderer);
        // }

        self.skull.draw(pass, &mut context.renderer);
        // pass.set_pipeline(&self.pipelines[0]);
        // pass.set_bind_group(1, Some(&self.light_binding.1), &[]);
        // self.cube.draw(pass, &mut context.renderer);
        //
        // pass.set_pipeline(&self.pipelines[1]);
        // self.wireframe.draw(pass, &mut context.renderer);
        // pass.set_pipeline(&self.pipelines[2]);
        // self.clickbait.draw(pass, &mut context.renderer);

        Ok(())
    }
    fn update(&mut self, context: &mut Context) -> anyhow::Result<()> {
        let win = context.renderer.window();
        if context
            .keyboard
            .just_pressed(winit::keyboard::KeyCode::KeyD)
        {
            let boo = !win.is_decorated();
            win.set_decorations(boo);
            win.set_cursor_hittest(boo).unwrap();
        }
        // dbg!(self.lights[0].position);
        // self.camera_controller
        //     .update_keyboard(&mut self.camera, &context.keyboard);
        // self.camera_controller
        //     .update_camera(&mut self.camera, &context.mouse, &context.keyboard);
        // maybe bundle these two lines into a Camera method that takes `&mut self, renderer: &mut Renderer`
        // self.camera.uniform.update_view_proj(self.camera.config());
        context.renderer.queue.write_buffer(
            &self.camera.buffer,
            0,
            bytemuck::cast_slice(&[self.camera.uniform]),
        );
        let elapsed = context.renderer.start.elapsed().as_secs_f32();
        // self.lights[0].position = [5. * SQRT_2 * elapsed.cos(), 5., 5. * SQRT_2 * elapsed.sin()];
        // context.renderer.queue.write_buffer(
        //     &self.light_binding.0,
        //     0,
        //     bytemuck::cast_slice(&self.lights),
        // );
        let s = win.inner_size();
        let horizontal = (-(s.width as f64) + context.mouse.mouse_position.x + 250.).atan2(1000.0);
        let vertical = (-(s.height as f64) + context.mouse.mouse_position.y + 250.).atan2(1000.0);
        self.skull.transform.rotation = glam::Quat::from_rotation_y(horizontal as f32)
            * glam::Quat::from_rotation_x(vertical as f32);
        self.cube.transform.rotation = glam::Quat::from_rotation_y(elapsed * PI);
        self.wireframe.transform.rotation = self.cube.transform.rotation;
        self.clickbait.transform = self.cube.transform;
        //self.emitter.update(context)?;
        Ok(())
    }
    fn post_process(
        &mut self,
        ctx: &mut Context,
        pass: &mut wgpu::RenderPass<'_>,
    ) -> anyhow::Result<(), wgpu::SurfaceError> {
        // pass.set_pipeline(&self.pipelines[3]);
        // pass.draw(0..3, 0..1);
        ctx.pass_post_processing(pass)
    }

    fn window_attributes() -> winit::window::WindowAttributes {
        WindowAttributes::default()
            .with_transparent(true)
            .with_active(false)
            .with_decorations(false)
            .with_has_shadow(false)
            .with_window_level(winit::window::WindowLevel::AlwaysOnTop)
            .with_inner_size(PhysicalSize::new(500, 500))

        // .with_visible(false)
    }
}

struct Clickbait {
    vertex_buffer: Buffer,
    instance_buffer: Buffer,
    index_buffer: Buffer,
    num_elements: u32,
    transform: Instance,
}

impl Clickbait {
    async fn from_mesh(
        mesh: &Mesh<impl Into<SimpleVertex<[f32; 3], StepInstance>> + VertexBuffer + Debug>,
        context: &mut Context,
    ) -> anyhow::Result<Self> {
        let vertices: Vec<SimpleVertex<[f32; 3], StepInstance>> =
            mesh.vertices.iter().map(|x| (*x).into()).collect();

        let offsets: &[SimpleVertex<_, StepInstance>] = &[
            [-1., -1.].into(),
            [1., -1.].into(),
            [-1., 1.].into(),
            [1., 1.].into(),
        ];

        let vertex_buffer = context
            .renderer
            .device
            .create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(offsets),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            });
        let instance_buffer = context
            .renderer
            .device
            .create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&vertices),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            });
        let index_buffer = context
            .renderer
            .device
            .create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: &mesh.indices,
                usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            });

        Ok(Self {
            vertex_buffer,
            instance_buffer,
            index_buffer,
            num_elements: mesh.vertices.len() as u32,
            transform: mesh.transform,
        })
    }
}

impl Renderable for Clickbait {
    fn draw(&self, pass: &mut wgpu::RenderPass, renderer: &mut Renderer) {
        self.draw_instances(pass, 0..self.num_elements, renderer);
    }
    fn draw_instances(
        &self,
        pass: &mut wgpu::RenderPass,
        instances: std::ops::Range<u32>,
        _: &mut Renderer,
    ) {
        pass.set_immediates(0, bytemuck::cast_slice(&[self.transform.to_raw()]));
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw(0..4, instances);
    }
}
