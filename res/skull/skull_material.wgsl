struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct MaterialProps {
    Ka: vec3<f32>,
    Kd: vec3<f32>,
    Ks: vec3<f32>,
    Ns: f32,
};

@group(1) @binding(0)
var material_tex: texture_2d<f32>;
@group(1) @binding(1)
var material_sampler: sampler;
@group(1) @binding(2)
var<uniform> material: MaterialProps;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.normal = normalize(model.normal);

    let world = vec4<f32>(model.position, 1.0);
    out.world_pos = world.xyz;

    out.clip_position = camera.view_proj * world;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(material_tex, material_sampler, in.tex_coords);

    // Simple lighting
    let light_dir = normalize(vec3<f32>(0.3, 0.8, 0.6));
    let norm = normalize(in.normal);

    let diff = max(dot(norm, light_dir), 0.0);

    // Specular
    let view_dir = normalize(camera.view_pos.xyz - in.world_pos);
    let half_dir = normalize(light_dir + view_dir);
    let spec = pow(max(dot(norm, half_dir), 0.0), material.Ns);

    let ambient = material.Ka;
    let diffuse = material.Kd * diff;
    let specular = material.Ks * spec;

    let lighting = ambient + diffuse + specular;

    return vec4<f32>(tex_color.rgb * lighting, tex_color.a);
}
