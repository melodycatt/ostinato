pub mod controller;
pub mod light;
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);

use std::{io::Write, sync::Arc};

use cgmath::{One, Point3, Quaternion, Vector3};
use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout, Device, RenderPass};

use crate::{mesh::{Mesh}, resources::Resource, State};

macro_rules! derive_camera_matrix {
    ($struct:ident) => {
        impl CameraMatrix for $struct {
            fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
                let forward = self.rotation * Vector3::new(0.0, 0.0, -1.0);
                let target = self.eye + forward;
                let up = self.rotation * Vector3::unit_y();
                let view = cgmath::Matrix4::look_at_rh(self.eye, target, up);
                let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

                return OPENGL_TO_WGPU_MATRIX * proj * view;
            }
        }
    };
}

pub trait CameraMatrix {
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32>;
}

#[derive(Debug)]
pub struct Camera {
    pub eye: Point3<f32>,
    pub rotation: Quaternion<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,

    pub uniform: CameraUniform,
    pub buffer: wgpu::Buffer,
    pub bind_group: Arc<wgpu::BindGroup>,
    pub bind_group_layout: Arc<wgpu::BindGroupLayout>,
//    pub camera_controller: CameraController,
}

#[derive(Clone, Copy)]
pub struct CameraConfig {
    pub eye: Point3<f32>,
    pub rotation: Quaternion<f32>,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}
impl CameraConfig {
    pub fn default() -> Self {
        Self {
            eye: (0.0, 0.0, 2.0).into(),
            rotation: Quaternion::one(),
            //aspect: config.width as f32 / config.height as f32,
            fovy: 60.0,
            znear: 0.1,
            zfar: 1000.0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct CameraData {
    eye: Point3<f32>,
    rotation: Quaternion<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}
impl From<(CameraConfig, f32)> for CameraData {
    fn from(value: (CameraConfig, f32)) -> Self {
        Self { eye: value.0.eye, rotation: value.0.rotation, aspect: value.1, fovy: value.0.fovy, znear: value.0.znear, zfar: value.0.zfar }
    }
}

impl Camera {
    pub fn new(config: CameraConfig, aspect: f32, device: &Device) -> Self {
        let config: CameraData = (config, aspect).into();
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(config);

        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }    
        );    

        let camera_bind_group_layout = Self::bind_group_layout(device);

        let binding = camera_buffer.as_entire_binding();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: binding,
                }   
            ],    
            label: Some("CAMERA"),
        });

        Self {
            eye: config.eye,
            rotation: config.rotation,
            aspect,
            fovy: config.fovy,
            znear: config.znear,
            zfar: config.zfar,
            uniform: camera_uniform,
            buffer: camera_buffer,
            bind_group: Arc::new(bind_group),
            bind_group_layout: Arc::new(camera_bind_group_layout),
        }
    }
    pub fn config(&self) -> CameraData {
        CameraData { eye: self.eye, rotation: self.rotation, aspect: self.aspect, fovy: self.fovy, znear: self.znear, zfar: self.zfar }
    }

    pub fn render(bind_group: Arc<BindGroup>, pass: &mut RenderPass, _excl_mirror: bool, state: &mut State) {
        //println!("1 {excl_mirror}");
        state.create_resource("bind_group::core::camera".into(), bind_group.clone());
        for (_, mesh) in state.world.query::<&Mesh>().iter() {
            let m = mesh.material(&state);
            pass.set_pipeline(&m.render_pipeline);
            for i in 0..m.global_bind_groups.len() {
                let b =  state.downcast_resource::<Arc<BindGroup>>(&m.global_bind_groups[i].1);
                //println!("{:?}, {b:#?}", m.global_bind_groups[i].1);
                pass.set_bind_group(m.global_bind_groups[i].0 as u32, Some(&**b), &[]);
            }
            for i in 0..m.bind_groups.len() {
                let b = &m.bind_groups[i];
                //println!("{b:#?}");
                pass.set_bind_group(b.0 as u32, Some(&b.1), &[]);
            }
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
        }
    }
    pub fn bind_group_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },    
                    count: None,
                }    
            ],    
            label: Some("camera_bind_group_layout"),
        })
    }
}

impl Resource for Camera {
    fn binding<'a>(&'a self) -> anyhow::Result<wgpu::BindingResource<'a>> {
        Ok(self.buffer.as_entire_binding())
    }
}

derive_camera_matrix!(Camera);
derive_camera_matrix!(CameraData);

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // We can't use cgmath with bytemuck directly, so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    pub view_pos: [f32; 4],
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_pos: [0.; 4],
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: CameraData) {
        self.view_pos = camera.eye.to_homogeneous().into();
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

