struct RaymarchUniform {
    time: f32,
    delta: f32,
    res: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> info: RaymarchUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) fragCoord: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );

    var out: VertexOutput;
    out.position = vec4<f32>(pos[vertex_index], 0.0, 1.0);
    out.fragCoord = (pos[vertex_index] * 0.5 + 0.5) * info.res;
    return out;
}

fn sphere_sdf(p: vec3<f32>, pos: vec3<f32>, radius: f32) -> f32 {
    return length(p - pos) - radius;
}

fn modf(x: vec3<f32>, y: f32) -> vec3<f32> {
    return x - y * floor(x/y);
}


// fractal 0
// fn de(p0: vec3<f32>) -> f32 {
//     var p = vec4<f32>(p0, 1.);
//     for(var i = 0u; i < 8u; i = i + 1u){
//         let nxyz = modf(p.xyz-1.,2.)-1.;
//         p.x = nxyz.x;
//         p.y = nxyz.y;
//         p.z = nxyz.z;
//         p*=1.4/dot(p.xyz,p.xyz);
//     }
//     return (length(p.xz/p.w)*0.25);
// }

// fractal 1
fn de(p: vec3<f32>) -> f32 {
	var Q: vec3<f32>;
	var i: f32 = 1.;
	var d: f32 = 1.;
	Q = p; 
    d = dot(sin(Q), cos(Q.yzx)) + 1.3;
    Q.x = Q.x + (3.14159265);
    d = min(d, dot(sin(Q), cos(Q.yzx)) + 1.3);
	Q.y = Q.y + (3.14159265);
	d = min(d, dot(sin(Q), cos(Q.yzx)) + 1.3);
	Q = Q * (30.);
	d = max(abs(d), (abs(dot(sin(Q), cos(Q.yzx)) + 1.3 - 1.3) - 0.5) / 30.);
	return d * 0.6;
} 

/*fn getRotZMat(a: f32) -> mat3x3<f32> {
	return mat3x3<f32>(cos(a), -sin(a), 0., sin(a), cos(a), 0., 0., 0., 1.);
} 

fn fractus(p: vec3<f32>) -> f32 {
	var z: vec2<f32> = p.xy;
	let c: vec2<f32> = vec2<f32>(0.28, -0.56) * cos(p.z * 0.1);
	var k: f32 = 1.;
	var h: f32 = 1.;

	for (var i: f32 = 0.; i < 8.; i = i + 1) {
		h = h * (4. * k);
		k = dot(z, z);
		if (k > 4.) {		break;
 }
		z = vec2<f32>(z.x * z.x - z.y * z.y, 2. * z.x * z.y) + c;
	}

	return sqrt(k / h) * log(k);
} 

fn de(p_in: vec3<f32>) -> f32 {
    var p = p_in;
    var e: f32 = 2.0;
    var v: f32 = 2.0;
    var u: f32;

    for (var j: i32 = 1; j <= 12; j += 1) {
        u = dot(p, p);
        v = v / u;
        p = p / u;
        p.y = 1.7 - p.y;

        if (j > 3) {
            let l = length(vec2<f32>(p.x, p.z) + length(p) / u * 0.55);
            e = min(e, l / v - 0.006);

            // p.xz = abs(p.xz) - 0.7;
            let pxz = vec2<f32>(p.x, p.z);
            let new_xz = abs(pxz) - vec2<f32>(0.7, 0.7);
            p = vec3<f32>(new_xz.x, p.y, new_xz.y);
        } else {
            // p = abs(p) - 0.86;
            p = abs(p) - vec3<f32>(0.86);
        }
    }

    return e;
}
*/

fn map(p: vec3<f32>) -> f32 {
    return de(p);
    /*let light_pos = vec3<f32>(
        3.0 * cos(info.time * 3.0),
        -1.0,
        3.0 * sin(info.time * 3.0)
    );
    let light = sphere_sdf(p, light_pos, 0.1);
    return min(light, min(sphere_sdf(p, vec3<f32>(1.0, 1.0, 1.0), 1.0), sphere_sdf(p, vec3<f32>(-3.0, -3.0, -3.0), 1.5)));*/
}

fn get_normal(p: vec3<f32>) -> vec3<f32> {
    let e = 0.0001;
    let x = vec3<f32>(e, 0.0, 0.0);
    let y = vec3<f32>(0.0, e, 0.0);
    let z = vec3<f32>(0.0, 0.0, e);
    return normalize(vec3<f32>(
        map(p + x) - map(p - x),
        map(p + y) - map(p - y),
        map(p + z) - map(p - z)
    ));
}

fn raymarch(ro: vec3<f32>, rd: vec3<f32>) -> f32 {
    var t = 0.0;
    for (var i = 0; i < 100; i++) {
        let p = ro + rd * t;
        let d = map(p);
        if (d < 0.0001) {
            return t;
        }
        if (t > 100.0) {
            break;
        }
        t += d;
    }
    return -1.0;
}

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

    let specular_strength = pow(max(dot(n, half_dir), 0.0), 32.0);
    let specular_color = specular_strength * light_color;

    let result = ambient_color + diffuse_color + specular_color;

    let a = min(1, 1 / (length(ro - p) * length(ro - p)));

    return vec4<f32>(result, 1.0);
}

fn ray_origin() -> vec3<f32> {
    let speed = 0.1;
    let d = 1.0;
    let theta = info.time * speed;
    let r = d * (2.0 + abs(cos(2.0 * theta)));
    return vec3<f32>(
        r * cos(theta), 
        -1.0, 
        r * sin(theta)
    );
}

fn ray_direction(ro: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let fov = 1.0;
    let targ = vec3<f32>(0);
    let forward = normalize(targ - ro);
    let up = vec3<f32>(0.0, 1.0, 0.0);
    let right = normalize(cross(forward, up));
    let corrected_up = cross(right, forward);

    // Apply field of view and uv offset to forward direction
    let fov_adjust = tan(fov * 0.5);

    let dir = normalize(
        forward +
        right * uv.x * fov_adjust +
        corrected_up * uv.y * fov_adjust
    );
    return dir;
}


@fragment
fn fs_main(@location(0) fragCoord: vec2<f32>) -> @location(0) vec4<f32> {
    let ro = ray_origin();
    let uv = (fragCoord / info.res) * 2.0 - vec2<f32>(1.0, 1.0);
    let rd = ray_direction(ro, uv);

    let t = raymarch(ro, rd);
    if (t < 0.) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let p = ro + rd * t;
    let n = get_normal(p);

    // Light 1 (moving)
    let light1 = ray_origin();

    // Light 2 (static)
    //let light2 = vec3<f32>(-2.0, 2.0, 1.0);

    let color1 = calculate_lighting(p, n, ro, light1) * vec4<f32>(0.8, 0.4, 0.4, 1.0);
    //let color2 = calculate_lighting(p, n, ro, light2);

    //let final_color = color1 + color2;

    //return color1;
    return select(vec4<f32>(0, 0, 0, 1), vec4<f32>(1), t < 100);
}
