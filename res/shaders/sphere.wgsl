struct TimeUniform {
    time: f32,
};

struct ResolutionUniform {
    res: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> time: TimeUniform;

@group(0) @binding(1)
var<uniform> resolution: ResolutionUniform;

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>( 3.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );

    let pos = positions[vertex_index];
    let uv = (pos + vec2<f32>(1.0, 1.0)) * 0.5;

    return VertexOutput(
        vec4<f32>(pos, 0.0, 1.0),
        uv
    );
}

fn sdSphere(p: vec3<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn mapScene(p: vec3<f32>) -> f32 {
    return map(p);
}

fn modv(x: vec2<f32>, y: f32) -> vec2<f32> {
    return x - y * floor(x / y);
}

fn map(p: vec3<f32>) -> f32 {
	let n = vec3<f32>(0, 1, 0);
	let k1 = 1.9;
	let k2 = (sin(p.x * k1) + sin(p.z * k1)) * 0.8;
	let k3 = (sin(p.y * k1) + sin(p.z * k1)) * 0.8;
	let w1 = 4.0 - dot(abs(p), normalize(n)) + k2;
	let w2 = 4.0 - dot(abs(p), normalize(n.yzx)) + k3;
	let s1 = length(modv(p.xy + vec2(sin((p.z + p.x) * 2.0) * 0.3, cos((p.z + p.x) * 1.0) * 0.5), 2.0) - 1.0) - 0.2;
	let s2 = length(modv(0.5+p.yz + vec2(sin((p.z + p.x) * 2.0) * 0.3, cos((p.z + p.x) * 1.0) * 0.3), 2.0) - 1.0) - 0.2;
	return min(w1, min(w2, min(s1, s2)));
}

fn rot(p: vec2<f32>, a: f32) -> vec2<f32> {
	return vec2<f32>(
		p.x * cos(a) - p.y * sin(a),
		p.x * sin(a) + p.y * cos(a));
}


/*fn mapScene(p: vec3<f32>) -> f32 {
    // Base sphere distance
    var d = sdSphere(p, 1.0);

    // Add periodic bumps using sin()
    let bumps = 0.1 // * sin(10.0 * p.x + time.time)
                    // * cos(10.0 * p.y + time.time)
                    * cos(10.0 * p.z + time.time)
                    ;

    d += bumps;

    return d;
}*/

fn getRayOrigin() -> vec3<f32> {
    //vec3 pos = vec3(0, 0, time);
    return vec3<f32>(0.0, 0.0, time.time);
}

fn getRayDirection(uv: vec2<f32>) -> vec3<f32> {
    let fov = 1.0;
    let aspect = resolution.res.x / resolution.res.y;
    let x = (uv.x * 2.0 - 1.0) * aspect;
    let y = 1.0 - uv.y * 2.0;
    return normalize(vec3<f32>(x * fov, y * fov, 1.0));
}

fn raymarch(ro: vec3<f32>, rd: vec3<f32>) -> vec2<f32> {
    var t = 0.0;
    let maxDist = 10000.0;
    let minDist = 0.001;
    let maxSteps = 100u;
    var d: f32;

    for (var i = 0u; i < maxSteps; i = i + 1u) {
        let p = ro + rd * t;
        d = mapScene(p);
        if d < minDist {
            break;
        }
        if t > maxDist {
            break;
        }
        t = t + d;
    }
    return vec2<f32>(t, d);
}

fn getNormal(p: vec3<f32>) -> vec3<f32> {
    let eps = 0.001;
    let dx = vec3<f32>(eps, 0.0, 0.0);
    let dy = vec3<f32>(0.0, eps, 0.0);
    let dz = vec3<f32>(0.0, 0.0, eps);

    return normalize(vec3<f32>(
        mapScene(p + dx) - mapScene(p - dx),
        mapScene(p + dy) - mapScene(p - dy),
        mapScene(p + dz) - mapScene(p - dz)
    ));
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let ro = getRayOrigin();
    var rd = getRayDirection(uv);
    var nxz = rot(rd.xz, time.time * 0.23);
    rd.x = nxz.x;
    rd.z = nxz.y;
    rd = rd.yzx;
	nxz = rot(rd.xz, time.time * 0.2);
    rd.x = nxz.x;
    rd.z = nxz.y;
    rd = rd.yzx;

    let m = raymarch(ro, rd);
    let dist = m.x;
    //let hit = dist < 1000.0;

    let hit_pos = ro + rd * dist;
    /*let normal = getNormal(hit_pos);
    let light_pos = vec3<f32>(2.0, 3.0, -1.0);
    let light_dir = normalize(light_pos - hit_pos);
    let dist_to_light = length(light_pos - hit_pos);
    let attenuation = 1.0 / (dist_to_light * dist_to_light);
    let diffuse = 10.0 * attenuation * max(dot(normal, light_dir), 0.0);
    let shade = select(0.7, diffuse, hit);*/

	let ip = hit_pos + rd * dist;
	var col = vec4<f32>(0.05*dist + 1 / vec3<f32>(dist * 1.) * abs(rd) +max(0.0, map(ip - 0.1) - m.y), 1.);
	//  col = sqrt(col);
	//var fragColor = vec4(0.05*dist+abs(rd) * col + max(0.0, map(ip - 0.1) - m.y), 1.0); //Thanks! Shane!
    //fragColor.a = 1.0 / (dist * dist * dist * dist);

    return col;
    // return vec4<f32>(vec3<f32>(shade), 1.0);
}
