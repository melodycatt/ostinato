struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};
@group(0) @binding(0)
var scene_tex: texture_2d<f32>;

@group(0) @binding(1)
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

    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let color = textureSample(scene_tex, scene_sampler, in.uv);

    let luminance = dot(color.rgb, vec3(0.299,0.587,0.114));

    return vec4(vec3(luminance),1.0);
}
