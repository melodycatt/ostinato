use wgpu::RenderPass;

use crate::State;

pub trait RenderObject {
    fn update(&mut self, _state: &mut State) {}
    fn render(&mut self, _pass: &mut RenderPass, _state: &State) {}
}