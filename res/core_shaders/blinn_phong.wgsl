struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};

struct Light {
    position  : vec3<f32>,
    _pad0     : f32,
    color     : vec3<f32>,
    intensity : f32,
};

struct Material {
    ambient   : vec3<f32>,
    _pad1     : f32,
    diffuse   : vec3<f32>,
    _pad2     : f32,
    specular  : vec3<f32>,
    shininess : f32,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) normal: vec3<f32>,
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
@group(1) @binding(0)
var<uniform> material: Material;

@group(0) @binding(0)
var<uniform> camera: CameraUniform;
@group(1) @binding(1)
var<storage, read> lights: array<Light>;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        transform.model0,
        transform.model1,
        transform.model2,
        transform.model3,
    );
    let normal_matrix = mat3x3<f32>(
        transform.normal0,
        transform.normal1,
        transform.normal2
    );
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.normal = normalize(normal_matrix * model.normal);
    var world_pos: vec4<f32> = model_matrix * vec4<f32>(model.position, 1.0);
    out.world_pos = world_pos.xyz;
    out.clip_position = camera.view_proj * world_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let N = normalize(in.normal);
    let V = normalize(camera.view_pos.xyz - in.world_pos);

    var result: vec3<f32> = material.ambient;

    let light_count = arrayLength(&lights);

    for (var i: u32 = 0u; i < light_count; i = i + 1u) {
        let light = lights[i];

        let L = normalize(light.position - in.world_pos);
        let H = normalize(L + V);

        // Distance attenuation (optional but recommended)
        let distance = length(light.position - in.world_pos);
        let attenuation = 1.0 / (distance * distance);

        // Ambient
        //let ambient = material.ambient * light.color * light.intensity;

        // Diffuse
        let diff = max(dot(N, L), 0.0);
        let diffuse = material.diffuse * diff * light.color * light.intensity;

        // Blinn-Phong Specular

        var spec: f32 = 0.0;
        if (diff > 0.0) {
            spec = pow(max(dot(N, H), 0.0), material.shininess);
        }
        let specular = material.specular * spec * light.color * light.intensity;

        result += (diffuse + specular) * attenuation;
    }

    return vec4<f32>(result, 1.0);
}
