use std::{
    f32::consts::{PI, SQRT_2},
    fmt::Debug,
    marker::PhantomData,
    ops::Range,
};

use anyhow::anyhow;
use ostinato::{
    AppHandler, Context,
    camera::light::LightUniform,
    prelude::*,
    renderer::Renderable,
    resources::{
        ModelVertex, SimpleVertex, StepInstance, StorageMesh, VertexBuffer, new_cube,
        pipeline::{BlinnPhong, LazyMaterial},
    },
};
use wgpu::{
    BindGroup, Buffer, BufferUsages, PrimitiveState,
    util::{BufferInitDescriptor, DeviceExt},
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
    // wireframe: StorageMesh,
    // clickbait: Clickbait,
    lights: Vec<LightUniform>,
}

impl AppHandler for ExampleHandler {
    async fn new(context: &mut Context) -> anyhow::Result<Self> {
        context.set_resource_directory(r"/Users/edwardlenzner/code/ostinato/res".to_owned());
        //j tjhis
        // let renderer = &mut context.renderer;

        let lights = vec![LightUniform::new([1., 1., 1.], [1., 1., 1.], 0.5)];
        let light_buffer =
            context
                .renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Lgiht buf"),
                    contents: bytemuck::cast_slice(&lights),
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                });
        BlinnPhong::get(context).resources[0].as_inner_buffer;

        let material = Material {
            diffuse: [0., 1., 0.],
            _pad0: 0.,
            ambient: [0., 0.1, 0.],
            _pad1: 0.,
            specular: [0., 1., 0.],
            shininess: 64.,
        };
        let material_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("mat buf"),
                    contents: bytemuck::cast_slice(&[material]),
                    usage: BufferUsages::UNIFORM,
                });
        renderer
            .shader_resources
            .insert("cube_material", material_buffer);

        let camera = ostinato::camera::Camera::new(
            ostinato::camera::CameraConfig {
                eye: (0.0, 0.0, 3.0).into(),
                rotation: glam::Quat::IDENTITY,
                fovy: 60.0,
                znear: 0.01,
                zfar: 1000.0,
            },
            renderer.config.width as f32 / renderer.config.height as f32,
            &renderer.device,
        );

        let _ = load_material(
            "core_shaders/blinn_phong",
            context,
            Some("core_shaders/blinn_phong"),
            Some(PrimitiveState {
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                ..Default::default()
            }),
        )
        .await?;

        let cube = new_cube(
            Instance {
                position: glam::Vec3::new(-0.5, -0.5, -0.5),
                pivot: glam::Vec3::new(0.5, 0.5, 0.5),
                rotation: glam::Quat::IDENTITY,
                scale: glam::Vec3::new(1., 1., 1.),
            },
            "core_shaders/blinn_phong",
            &mut context.renderer,
        );
        let mut wireframe = StorageMesh::from_mesh(cube.clone(), &context.renderer.device)?;
        wireframe.material = load_material(
            "core_shaders/wireframe",
            context,
            None,
            Some(PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                cull_mode: None,
                ..Default::default()
            }),
        )
        .await?;
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
        })
    }
    fn render(
        &mut self,
        context: &mut Context,
        pass: &mut wgpu::RenderPass<'_>,
    ) -> anyhow::Result<(), wgpu::SurfaceError> {
        // TODO this looks like boilerplate!!!!!! stupid!!!!!!!! lets change that
        // context.renderer.render_with_camera(pass, &mut self.camera.as_mut().unwrap(), &self.cube.as_ref().unwrap()).expect("AAA");
        // context.renderer.render_with_camera(pass, &mut self.camera.as_mut().unwrap(), &self.cube2.as_ref().unwrap()).expect("AAA");
        //context.renderer.render_with_camera(pass, &mut self.camera.as_mut().unwrap(), &self.skull.as_ref().unwrap().meshes[0]).expect("AAA");
        //context.renderer.render_with_camera(pass, &mut self.camera.as_mut().unwrap(), &self.skull.as_ref().unwrap().meshes[1]).expect("AAA");
        //context.renderer.render_with_camera(pass, &mut self.camera.as_mut().unwrap(), &self.skull.as_ref().unwrap().meshes[2]).expect("AAA");
        //context.renderer.render_with_camera(pass, &mut self.camera.as_mut().unwrap(), &self.skull.as_ref().unwrap().meshes[3]).expect("AAA");
        context.renderer.set_camera(&self.camera);
        /*self.wireframe
        .draw(pass, &[], &mut context.renderer)
        .unwrap();*/
        self.clickbait
            .draw(pass, &[], &mut context.renderer)
            .unwrap();
        self.cube.draw(pass, &[], &mut context.renderer).unwrap();
        Ok(())
    }
    fn update(&mut self, context: &mut Context) -> anyhow::Result<()> {
        //dbg!(self.camera_controller.pitch, self.camera_controller.yaw);
        self.camera_controller
            .update_camera(&mut self.camera, &context.mouse, &context.keyboard);
        // maybe bundle these two lines into a Camera method that takes `&mut self, renderer: &mut Renderer`
        self.camera.uniform.update_view_proj(self.camera.config());
        context.renderer.queue.write_buffer(
            &self.camera.buffer,
            0,
            bytemuck::cast_slice(&[self.camera.uniform]),
        );
        let elapsed = context.start.elapsed().as_secs_f32();
        self.lights[0].position = [SQRT_2 * elapsed.cos(), 1., SQRT_2 * elapsed.sin()];
        context.renderer.queue.write_buffer(
            context
                .renderer
                .shader_resources
                .get("lights")
                .unwrap()
                .as_inner_buffer(),
            0,
            bytemuck::cast_slice(&self.lights),
        );
        self.cube.transform.rotation = glam::Quat::from_rotation_y(elapsed * PI);
        self.wireframe.transform.rotation = self.cube.transform.rotation;
        self.clickbait.transform = self.cube.transform;
        //self.emitter.update(context)?;
        Ok(())
    }
}

struct Clickbait {
    vertex_buffer: Buffer,
    instance_buffer: Buffer,
    index_buffer: Buffer,
    num_elements: u32,
    material: usize,
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
            material: load_material(
                "core_shaders/clickbait",
                context,
                None,
                Some(PrimitiveState {
                    cull_mode: None,
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                }),
            )
            .await?,
            transform: mesh.transform,
        })
    }
}

impl Renderable for Clickbait {
    fn draw(
        &self,
        pass: &mut wgpu::RenderPass,
        manual_bindings: &[BindGroup],
        renderer: &mut Renderer,
    ) -> anyhow::Result<()> {
        self.draw_instances(pass, manual_bindings, 0..self.num_elements, renderer)
    }
    fn draw_instances(
        &self,
        pass: &mut wgpu::RenderPass,
        manual_bindings: &[BindGroup],
        instances: Range<u32>,
        renderer: &mut Renderer,
    ) -> anyhow::Result<()> {
        let m = renderer
            .materials
            .get(self.material)
            .ok_or(anyhow!("x_x :: todo write this panic message"))?;
        pass.set_pipeline(&m.render_pipeline);
        pass.set_bind_group(0, m.bind_groups[0].as_ref(), &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.set_immediates(0, bytemuck::bytes_of(&self.transform.to_raw()));
        //println!("drawing 0..{}", self.num_elements);
        pass.draw(0..4, instances);
        Ok(())
    }
}
