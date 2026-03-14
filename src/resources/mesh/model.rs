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
        context: &mut crate::prelude::Context,
    ) -> anyhow::Result<()> {
        for i in self.meshes.iter() {
            i.draw(pass, context)?
        }
        Ok(())
    }
    fn draw_instances(
        &self,
        pass: &mut wgpu::RenderPass,
        instances: std::ops::Range<u32>,
        context: &mut crate::prelude::Context,
    ) -> anyhow::Result<()> {
        for i in self.meshes.iter() {
            i.draw_instances(pass, instances.clone(), context)?
        }
        Ok(())
    }
}
