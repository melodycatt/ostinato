struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0) // 1.
var<uniform> camera: CameraUniform;
@group(1) @binding(0) var img: texture_2d<f32>;
@group(1) @binding(1) var smp: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.position = camera.view_proj * vec4<f32>(model.position, 1.0);
    //out.position = vec4(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(img, smp, in.tex_coords);
}