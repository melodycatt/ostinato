struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0) // 1.
var<uniform> camera: CameraUniform;

struct Light {
    light_pos: vec3<f32>,
    brightness: f32,
};
@group(1) @binding(0) // 1.
var<uniform> light: Light;

fn calculate_lighting(
    p: vec3<f32>,
    n: vec3<f32>,
    ro: vec3<f32>,
    light_pos: vec3<f32>
) -> vec4<f32> {
    let light_color = vec3<f32>(1.0, 1.0, 1.0);
    // We don't need (or want) much ambient light, so 0.1 is fine
    let ambient_strength = 0.1;
    let ambient_color = light_color * ambient_strength;

    let light_dir = normalize(light_pos - p);
    let view_dir = normalize(ro - p);
    let half_dir = normalize(view_dir + light_dir);

    let diffuse_strength = max(dot(n, light_dir), 0.0);
    let diffuse_color = light_color * diffuse_strength;

    let specular_strength = pow(max(dot(n, half_dir), 0.0), 32.0) * 0.5;
    let specular_color = specular_strength * light_color;


    let result = ambient_color + diffuse_color + specular_color;

    //let a = min(1, light.brightness / (length(ro - p) * length(ro - p)));
    let dist = length(light_pos - p);
    let attenuation = 1.0 / (1.0 + 0.1 * dist + 0.02 * dist * dist);

    return vec4<f32>(result * attenuation, 1.0);
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_pos: vec3<f32>,
    @location(2) normal: vec3<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_pos = camera.view_pos.xyz;

    let col = calculate_lighting(in.world_pos, in.normal, camera.view_pos.xyz, light_pos);
    return col;
}