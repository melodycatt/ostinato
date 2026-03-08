struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};

struct Transform {
    model0: vec4<f32>,
    model1: vec4<f32>,
    model2: vec4<f32>,
    model3: vec4<f32>,
    normal0: vec3<f32>,
    _pad0: f32,
    normal1: vec3<f32>,
    _pad1: f32,
    normal2: vec3<f32>,
    _pad2: f32,
};

var<immediate> transform: Transform; // SIZE: 64

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VsIn {
    @location(1) position: vec3<f32>,   // vertex buffer (per-instance)
    @location(0) corner: vec2<f32>,     // quad corner (-1..1)
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) local: vec2<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    let model_matrix = mat4x4<f32>(
        transform.model0,
        transform.model1,
        transform.model2,
        transform.model3,
    );

    let radius: f32 = 0.03;

    // world → clip
    let center_clip = camera.view_proj * model_matrix * vec4<f32>(in.position, 1.0);

    // offset in clip space (screen-aligned quad)
    let offset = vec4<f32>(in.corner * radius, 0.0, 0.0);

    out.clip_pos = center_clip + offset;
    out.local = in.corner;

    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // circle mask
    if (length(in.local) > 1.0) {
        discard;
    }

    return vec4<f32>(1.0, 0.2, 0.2, 1.0);
}
