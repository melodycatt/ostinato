fn hash(p: vec4<f32>) -> f32 {
    let h = dot(p, vec4<f32>(12.9898, 78.233, 34.852, 57.893));
    return fract(sin(h) * 43758.5453123);
}

struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0) // 1.
var<uniform> camera: CameraUniform;
struct TimeUniform {
    time: f32,
};
@group(1) @binding(0) // 1.
var<uniform> time: TimeUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    out.position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    /*let px = floor(in.position.xy * 0.1);
    //let px = in.position.xy;
    let l = hash33(vec3<f32>(px, time.time * 60.0));

    let out = vec4<f32>(l,l,l,1.0);
    return out;*/
    let uvw = in.position.xyz * 0.5 + 0.5;
    let speed = 10.0; // speed of static movement
    //let speed_time = time.time * speed;
    let scale = 0.001; // size of the noise grain
    let sin_time = sin(time.time);
    let pixel_size = sin_time * sin_time * 20.0 + 3.0; // size of the noise grain
    let n = hash(vec4<f32>(floor(uvw / pixel_size) * pixel_size, time.time * speed) * scale);
    return vec4<f32>(n, n, n, 1.0);
}