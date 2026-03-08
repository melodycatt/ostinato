use std::fmt::Debug;

use wgpu::BindGroup;

use crate::{
    renderer::Renderable,
    resources::{InstancedMesh, VertexBuffer},
};

// todo ?
// model.rs
pub struct Model<V: VertexBuffer + Debug> {
    pub meshes: Vec<InstancedMesh<V>>,
}

impl<V: VertexBuffer + Debug> Renderable for Model<V> {
    fn draw(
        &self,
        pass: &mut wgpu::RenderPass,
        manual_bindings: &[BindGroup],
        renderer: &mut crate::prelude::Renderer,
    ) -> anyhow::Result<()> {
        for i in self.meshes.iter() {
            i.draw(pass, manual_bindings, renderer)?
        }
        Ok(())
    }
    fn draw_instances(
        &self,
        pass: &mut wgpu::RenderPass,
        manual_bindings: &[BindGroup],
        instances: std::ops::Range<u32>,
        renderer: &mut crate::prelude::Renderer,
    ) -> anyhow::Result<()> {
        for i in self.meshes.iter() {
            i.draw_instances(pass, manual_bindings, instances.clone(), renderer)?
        }
        Ok(())
    }
}
