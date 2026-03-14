#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub position: [f32; 3],
    radius: f32,
    pub color: [f32; 3],
    pub intensity: f32,
}
impl LightUniform {
    pub fn new(position: [f32; 3], color: [f32; 3], intensity: f32) -> Self {
        let max_c = color[0].max(color[1]).max(color[2]);
        let radius = ((max_c * intensity) / 0.001).sqrt();
        Self {
            color,
            position,
            intensity,
            radius,
        }
    }
}
