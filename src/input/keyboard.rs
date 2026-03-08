use std::collections::HashSet;
use winit::{
    event::{DeviceEvent, DeviceId, ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
};

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

    pub fn is_pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&PhysicalKey::Code(key))
    }
    pub fn just_pressed(&self, key: PhysicalKey) -> bool {
        //println!("AAAA {key:?} {} {}", self.pressed.contains(&key), self.prev_pressed.contains(&key));
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
        if let WindowEvent::KeyboardInput {
            device_id: _,
            event,
            is_synthetic: _,
        } = event
        {
            match event.state {
                ElementState::Pressed => self.pressed.insert(event.physical_key),
                ElementState::Released => self.pressed.remove(&event.physical_key),
            };
        }
    }

    pub fn _device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        _event: &DeviceEvent,
    ) {
    }
}

impl Default for KeyboardData {
    fn default() -> Self {
        Self::new()
    }
}
