use std::collections::HashSet;

use winit::{event::{DeviceEvent, DeviceId, ElementState, WindowEvent}, event_loop::ActiveEventLoop, keyboard::PhysicalKey};

#[derive(Clone, Debug)]
pub struct KeyboardData {
    // pressed keys
    pressed: HashSet<PhysicalKey>,
    prev_pressed: HashSet<PhysicalKey>,
}

impl KeyboardData {
    pub fn new() -> Self {
        Self {
            pressed: HashSet::with_capacity(256),
            prev_pressed: HashSet::with_capacity(256),
        }
    }

    pub fn update(&mut self) {
        self.prev_pressed = self.pressed.clone();
    }

    pub fn is_pressed(&self, key: PhysicalKey) -> bool {
        self.pressed.contains(&key)
    }
    pub fn just_pressed(&self, key: PhysicalKey) -> bool {
        self.pressed.contains(&key) && !self.prev_pressed.contains(&key)
    }
    pub fn just_released(&self, key: PhysicalKey) -> bool {
        !self.pressed.contains(&key) && self.prev_pressed.contains(&key)
    }

    pub fn window_event(        
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: &WindowEvent,
    ) {
        match event {
            WindowEvent::KeyboardInput { device_id: _, event, is_synthetic: _ } => {
                match event.state {
                    ElementState::Pressed => self.pressed.insert(event.physical_key),
                    ElementState::Released => self.pressed.remove(&event.physical_key),
                };
            }
            _ => {}
        }
    }

    pub fn _device_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _device_id: DeviceId,
            event: &DeviceEvent,
        ) {
        match event {
            _ => {}
            //DeviceEvent::MouseWheel { delta }
        }
    }
}