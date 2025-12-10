use std::collections::HashSet;

use crate::{Resource, resources::Resource};
use winit::{event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent}, event_loop::ActiveEventLoop};

#[derive(Clone, Debug, Resource)]
pub struct MouseData {
    // mouse delta over one frame, in pixels
    pub delta: [f64; 2],
    // pressed mousebuttons
    pressed: HashSet<MouseButton>,
    prev_pressed: HashSet<MouseButton>,
    // is the cursor inside the window? (initialises to false, and only updates when the cursor first enters or leaves the window)
    pub cursor_inside: bool,
    // scroll delta over one fram, in pixels
    pub scroll_delta: [f64; 2]
}

impl MouseData {
    pub fn new() -> Self {
        Self {
            delta: [0., 0.],
            pressed: HashSet::with_capacity(256),
            prev_pressed: HashSet::with_capacity(256),
            cursor_inside: false,
            scroll_delta: [0.; 2]
        }
    }

    pub fn update(&mut self) {
        self.prev_pressed = self.pressed.clone();
        self.delta = [0.; 2];
        self.scroll_delta = [0.; 2];
    }

    pub fn is_pressed(&self, button: MouseButton) -> bool {
        self.pressed.contains(&button)
    }
    pub fn just_pressed(&self, button: MouseButton) -> bool {
        self.pressed.contains(&button) && !self.prev_pressed.contains(&button)
    }
    pub fn just_released(&self, button: MouseButton) -> bool {
        !self.pressed.contains(&button) && self.prev_pressed.contains(&button)
    }

    pub fn window_event(        
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: &WindowEvent,
    ) {
        match event {
            WindowEvent::CursorEntered { device_id: _ } => self.cursor_inside = true,
            WindowEvent::CursorLeft { device_id: _ } => self.cursor_inside = false,
            WindowEvent::MouseInput { device_id: _, state, button } => {
                match state {
                    ElementState::Pressed => self.pressed.insert(*button),
                    ElementState::Released => self.pressed.remove(button),
                };
            },
            WindowEvent::MouseWheel { device_id: _, delta, phase: _ } => {
                match delta {
                    MouseScrollDelta::LineDelta(x, y) => { 
                        self.scroll_delta[0] += *x as f64 * 20.;
                        self.scroll_delta[1] += *y as f64 * 20.; 
                    },
                    MouseScrollDelta::PixelDelta(pos) => {
                        self.scroll_delta[0] += pos.x;
                        self.scroll_delta[1] += pos.y; 
                    }
                }
            },
            _ => {}
        }
    }

    pub fn device_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _device_id: DeviceId,
            event: &DeviceEvent,
        ) {
        match event {
            DeviceEvent::MouseMotion{ delta } => {
                self.delta[0] += delta.0;
                self.delta[1] += delta.1; 
            },
            _ => {}
            //DeviceEvent::MouseWheel { delta }
        }
    }
}