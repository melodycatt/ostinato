use bytemuck::{Pod, Zeroable};
use ostinato::{AppHandler, renderer::post_pipeline, resources::load_pipeline};
use wgpu::RenderPipeline;

fn main() {
    ostinato::run::<Mandelbrot>().unwrap();
}

struct Mandelbrot {
    pipeline: RenderPipeline,
    immediates: Immediates,
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ImmediatesRaw {
    center: [[f32; 2]; 2],
    zoom: [f32; 2],
}
struct Immediates {
    center: [f64; 2],
    zoom: f64,
}
impl Immediates {
    fn to_raw(&self) -> ImmediatesRaw {
        ImmediatesRaw {
            center: [split_f64(self.center[0]), split_f64(self.center[1])],
            zoom: split_f64(self.zoom),
        }
    }
}

impl AppHandler for Mandelbrot {
    async fn new(context: &mut ostinato::Context) -> anyhow::Result<Self> {
        context.set_resource_directory(r"/Users/edwardlenzner/code/ostinato/res".to_owned());
        let p = post_pipeline("core_shaders/post_processing/mandelbrot.wgsl", 24, context);
        Ok(Self {
            pipeline: p,
            immediates: Immediates {
                zoom: 0.01,
                center: [-0.5, 0.],
            },
        })
    }

    fn render(
        &mut self,
        context: &mut ostinato::Context,
        pass: &mut wgpu::RenderPass<'_>,
    ) -> anyhow::Result<(), wgpu::SurfaceError> {
        Ok(())
    }
    fn update(&mut self, context: &mut ostinato::Context) -> anyhow::Result<()> {
        if context.mouse.scroll_delta[1] > 0. {
            self.immediates.zoom *= 0.935;
        }
        if context.mouse.scroll_delta[1] < 0. {
            self.immediates.zoom *= 1.065;
        }
        if context.mouse.is_pressed(winit::event::MouseButton::Left) {
            self.immediates.center[0] -= context.mouse.delta[0] * self.immediates.zoom;
            self.immediates.center[1] -= context.mouse.delta[1] * self.immediates.zoom;
        }
        Ok(())
    }
    fn post_process(
        &mut self,
        context: &mut ostinato::Context,
        pass: &mut wgpu::RenderPass<'_>,
    ) -> anyhow::Result<(), wgpu::SurfaceError> {
        pass.set_pipeline(&self.pipeline);
        pass.set_immediates(0, bytemuck::bytes_of(&self.immediates.to_raw()));
        pass.draw(0..3, 0..1);
        Ok(())
    }
}

// Splits a single f64 into a high and low f32 pair
fn split_f64(v: f64) -> [f32; 2] {
    // Cast to f32 to grab the most significant bits (the "high" part)
    let hi = v as f32;
    // Subtract the high part from the original to find the lost precision (the "low" part)
    let lo = (v - hi as f64) as f32;
    [hi, lo]
}
