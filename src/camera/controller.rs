use std::f32::consts::{FRAC_PI_2, PI};

use cgmath::{Quaternion, Rad, Rotation3, Vector3};
use crate::Resource;
use winit::{keyboard::{KeyCode}};

use crate::{camera::{Camera}, input::{keyboard::KeyboardData, mouse::MouseData}};

#[derive(Debug, Resource)]
pub struct CameraController {
    pub speed: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            pitch: 0.0,
            yaw: PI
        }
    }

    /*pub fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    KeyCode::KeyW | KeyCode::ArrowUp => {
                        self.is_forward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyA | KeyCode::ArrowLeft => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyS | KeyCode::ArrowDown => {
                        self.is_backward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyD | KeyCode::ArrowRight => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            },
            _ => false,
        }
    }*/

    pub fn update_camera(&mut self, camera: &mut Camera, mouse: &MouseData, keyboard: &KeyboardData) {
        //use cgmath::InnerSpace;
        //println!("{:?}", mouse.delta);
        if keyboard.is_pressed(KeyCode::KeyW.into())
        || keyboard.is_pressed(KeyCode::ArrowUp.into()) {
            camera.eye += camera.rotation * Vector3 { x: 0.0, y: 0.0, z: -self.speed };
        }
        if keyboard.is_pressed(KeyCode::KeyS.into())
        || keyboard.is_pressed(KeyCode::ArrowDown.into()) {
            camera.eye += camera.rotation * Vector3 { x: 0.0, y: 0.0, z: self.speed };
        }
        if keyboard.is_pressed(KeyCode::KeyA.into())
        || keyboard.is_pressed(KeyCode::ArrowLeft.into()) {
            camera.eye += camera.rotation * Vector3 { x: -self.speed, y: 0.0, z: 0.0 };
        }
        if keyboard.is_pressed(KeyCode::KeyD.into())
        || keyboard.is_pressed(KeyCode::ArrowRight.into()) {
            camera.eye += camera.rotation * Vector3 { x: self.speed, y: 0.0, z: 0.0 };
        }
        if keyboard.is_pressed(KeyCode::KeyE.into()) {
            camera.eye += Vector3 { y: self.speed, x: 0.0, z: 0.0 };
        }
        if keyboard.is_pressed(KeyCode::KeyQ.into()) {
            camera.eye += Vector3 { y: -self.speed, x: 0.0, z: 0.0 };
        }

        self.yaw -= mouse.delta.x as f32 * 0.003;
        self.pitch -= mouse.delta.y as f32 * 0.003;
        self.pitch = self.pitch.clamp(-FRAC_PI_2, FRAC_PI_2);
        self.yaw %= 2.0 * PI;

        camera.rotation = Quaternion::from_angle_y(Rad(self.yaw)) * Quaternion::from_angle_x(Rad(self.pitch));
    }
}
