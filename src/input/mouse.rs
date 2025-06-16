use std::collections::HashSet;

use vectors::Vector2;
use winit::{event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent}, event_loop::ActiveEventLoop};

#[derive(Clone, Debug)]
pub struct MouseData {
    // mouse delta over one frame, in pixels
    pub delta: Vector2<f64>,
    // pressed mousebuttons
    pressed: HashSet<MouseButton>,
    prev_pressed: HashSet<MouseButton>,
    // is the cursor inside the window? (initialises to false, and only updates when the cursor first enters or leaves the window)
    pub cursor_inside: bool,
    // scroll delta over one fram, in pixels
    pub scroll_delta: Vector2<f64>
}

impl MouseData {
    pub fn new() -> Self {
        Self {
            delta: Vector2::identity(),
            pressed: HashSet::with_capacity(256),
            prev_pressed: HashSet::with_capacity(256),
            cursor_inside: false,
            scroll_delta: Vector2::identity()
        }
    }

    pub fn update(&mut self) {
        self.prev_pressed = self.pressed.clone();
        self.delta = Vector2::identity();
        self.scroll_delta = Vector2::identity()
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
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: &WindowEvent,
    ) {
        match event {
            WindowEvent::CursorEntered { device_id } => self.cursor_inside = true,
            WindowEvent::CursorLeft { device_id } => self.cursor_inside = false,
            WindowEvent::MouseInput { device_id, state, button } => {
                match state {
                    ElementState::Pressed => self.pressed.insert(*button),
                    ElementState::Released => self.pressed.remove(button),
                };
            },
            WindowEvent::MouseWheel { device_id, delta, phase } => {
                match delta {
                    MouseScrollDelta::LineDelta(x, y) => { self.scroll_delta += <(f64, f64) as Into<Vector2<f64>>>::into((*x as f64, *y as f64)) * 20.0; },
                    MouseScrollDelta::PixelDelta(pos) => { self.scroll_delta += <(f64, f64) as Into<Vector2<f64>>>::into((pos.x, pos.y)); }
                }
            },
            _ => {}
        }
    }

    pub fn device_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            device_id: DeviceId,
            event: &DeviceEvent,
        ) {
        match event {
            DeviceEvent::MouseMotion{ delta } => { println!("!!!"); self.delta += (*delta).into() },
            _ => {}
            //DeviceEvent::MouseWheel { delta }
        }
    }
}