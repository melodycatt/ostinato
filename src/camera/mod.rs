pub mod controller;
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);

use std::sync::Arc;

use cgmath::{One, Point3, Quaternion, Vector3};
use wgpu::{util::DeviceExt, Device};

use crate::resources::Resource;

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
    //camera_bind_group: wgpu::BindGroup,
    pub bind_group_layout: Arc<wgpu::BindGroupLayout>,
//    pub camera_controller: CameraController,
}

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

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },    
                    count: None,
                }    
            ],    
            label: Some("camera_bind_group_layout"),
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
            bind_group_layout: Arc::new(camera_bind_group_layout)
        }
    }
    pub fn config(&self) -> CameraData {
        CameraData { eye: self.eye, rotation: self.rotation, aspect: self.aspect, fovy: self.fovy, znear: self.znear, zfar: self.zfar }
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
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: CameraData) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

