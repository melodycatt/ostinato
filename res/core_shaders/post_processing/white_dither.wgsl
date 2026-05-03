struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct Uniform {
    time: f32,
    res: vec2<f32> 
};
@group(0) @binding(0)
var<uniform> post_uniform: Uniform;
@group(1) @binding(0)
var scene_tex: texture_2d<f32>;

@group(1) @binding(1)
var scene_sampler: sampler;
@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VsOut {
    var positions = array<vec2<f32>,3>(
        vec2(-1.0,-1.0),
        vec2( 3.0,-1.0),
        vec2(-1.0, 3.0)
    );

    var out: VsOut;

    let p = positions[i];

    out.pos = vec4(p,0.0,1.0);
    out.uv = p * 0.5 + 0.5;
    out.uv.y = 1.0 - out.uv.y;

    return out;
}

fn bayer4(x: i32, y: i32) -> f32 {
    let m = array<f32,16>(
        0.0,  8.0,  2.0, 10.0,
        12.0, 4.0, 14.0, 6.0,
        3.0, 11.0,  1.0,  9.0,
        15.0, 7.0, 13.0,  5.0
    );

    return m[(y & 3) * 4 + (x & 3)] / 16.0;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let pixel_size = 2.0;

    // pixelate the screen sampling
    let uv = floor(in.uv * post_uniform.res / pixel_size) * pixel_size / post_uniform.res;

    let color = textureSample(scene_tex, scene_sampler, uv);

    // compute pixel coords from the pixelated uv
    let px = i32(uv.x * post_uniform.res.x) / 3;
    let py = i32(uv.y * post_uniform.res.y) / 3;

    let threshold = bayer4(px, py);

    let levels = 4.0;

    let dithered = floor(color.rgb * levels + threshold) / levels;

    return vec4(dithered, color.a);

    //varreturn vec4<f32> (0.0, 0.0, 0.0, 0.0);
}
