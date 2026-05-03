struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};

struct Immediates {
    transform: Transform,
    color: vec4<f32>
}
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


var<immediate> immediates : Immediates;
//var<immediate> transform: Transform; // SIZE: 112 bytes

//var<immediate> color : Color;

@binding(0) @group(0) var<uniform> camera: CameraUniform;
@binding(0) @group(1) var<storage, read> positions : array<f32>;
@binding(1) @group(1) var<storage, read> indices   : array<u32>;

struct VertexInput {
	@builtin(instance_index) instanceID : u32,
	@builtin(vertex_index) vertexID : u32,
};

struct VertexOutput {
	@builtin(position) position : vec4<f32>,
};

@vertex
fn vs_main(vertex : VertexInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        immediates.transform.model0,
        immediates.transform.model1,
        immediates.transform.model2,
        immediates.transform.model3,
    );

	var localToElement = array<u32, 6>(0u, 1u, 1u, 2u, 2u, 0u);

	var triangleIndex = vertex.vertexID / 6u;
	var localVertexIndex = vertex.vertexID % 6u;

	var elementIndexIndex = 3u * triangleIndex + localToElement[localVertexIndex];
	var elementIndex = indices[elementIndexIndex];

	var position = vec4<f32>(
		positions[3u * elementIndex + 0u],
		positions[3u * elementIndex + 1u],
		positions[3u * elementIndex + 2u],
		1.0
	);

	position = camera.view_proj * (model_matrix * position);

	var output : VertexOutput;
	output.position = position;

	return output;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(immediates.color.xyz, 1.0);
}
