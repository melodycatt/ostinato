@group(1) @binding(0) var img: texture_2d<f32>;
@group(1) @binding(1) var smp: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) normal: vec3<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    //return vec4<f32>(0, 0, 0, 0);
    return textureSample(img, smp, in.tex_coords);
}