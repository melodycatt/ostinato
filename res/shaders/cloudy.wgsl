// Created by Stephane Cuillerdier - Aiekick/2015
// License Creative Commons Attribution-NonCommercial-ShareAlike 3.0 Unported License.
// Tuned via XShade (http://www.funparadigm.com/xshade/)

/* 
	variation more cloudy off Another Cloudy Tunnel : 
		https://www.shadertoy.com/view/4lSXRK

	the cloudy famous tech come from the shader of duke : https://www.shadertoy.com/view/MljXDw
        Himself a Port of a demo by Las => http://www.pouet.net/topic.php?which=7920&page=29&x=14&y=9
*/

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
@group(1) @binding(0) var ic0tex: texture_2d<f32>;
@group(1) @binding(1) var ic0sampler: sampler;

fn cosPath(p: vec3<f32>, dec: vec3<f32>) -> f32 {return dec.x * cos(p.z * dec.y + dec.z);}
fn sinPath(p: vec3<f32>, dec: vec3<f32>) -> f32 {return dec.x * sin(p.z * dec.y + dec.z);}

fn getCylinder(p: vec3<f32>, pos: vec2<f32>, r: f32, c: vec3<f32>, s: vec3<f32>) -> vec2<f32>
{
	return p.xy - pos - vec2<f32>(cosPath(p, c), sinPath(p, s));
}

/////////////////////////
// FROM Shader Cloudy spikeball from duke : https://www.shadertoy.com/view/MljXDw
fn pn(x: vec3<f32>) -> f32
{
    let p = floor(x);
    var f = fract(x);
	f = f*f*(3.0-2.0*f);
	let uv = (p.xy+vec2<f32>(37.0,17.0)*p.z) + f.xy;
	let rg = textureSampleLevel(ic0tex, ic0sampler, (uv + vec2<f32>(0.5)) / 256.0, 0.0).yx;
	return -1.0+2.4*mix( rg.x, rg.y, f.z );
}

fn fpn(p2: vec3<f32>) -> f32
{
    let p = p2 + (time.time * 2.5)*5.;
	return pn(p*0.02)*1.98 + pn(p*0.02)*0.62 + pn(p*0.09)*0.39;
}
/////////////////////////

fn map(p: vec3<f32>) -> f32
{
	let pnNoise = fpn(p*13.)*.8;
	let path = sinPath(p, vec3<f32>(6.2, .33, 0.));
	let bottom = p.y + pnNoise;
	var cyl = 0.;
	var vecOld =vec2<f32>(0., 0.);
	for (var i=0.;i<6.;i = i + 1.)
	{
		let x = 1. * i;
		let y = .88 + 0.0102*i;
		let z = -0.02 -0.16*i;
		let r = 4.4 + 2.45 * i;
		let v = getCylinder(p, vec2<f32>(path, 3.7 * i), r , vec3<f32>(x,y,z), vec3<f32>(z,x,y));
		cyl = r - min(length(v), length(vecOld));
		vecOld = v;	
	}
	cyl = cyl + pnNoise;
	cyl = min(cyl, bottom);
	return cyl;
}

fn cam(uv: vec2<f32>, ro: vec3<f32>, cu: vec3<f32>, cv: vec3<f32>) -> vec3<f32>
{
	let rov = normalize(cv-ro);
    let u =  normalize(cross(cu, rov));
    let v =  normalize(cross(rov, u));
	let fov = 3.;
    let rd = normalize(rov + fov*u*uv.x + fov*v*uv.y);
    return rd;
}

struct VertexOutput {
	@builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Vertex shader: output position and UV coords
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>( 3.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );

    let pos = positions[vertex_index];
    let uv = (pos.xy + vec2<f32>(1.0, 1.0)) * 10.0; // Map from [-1,1] to [0,1]
    
    var out: VertexOutput;
	out.pos = vec4<f32>(pos, 0.0, 1.0);
	out.uv = uv;

    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let t = time.time*2.5;
	var f = vec4(0,0.15,0.32,1);
	let si = resolution.res.xy;
	let uv = (2.*input.uv-si)/min(si.x, si.y);
    var ro = vec3<f32>(0., 0., 0.);
	var p = vec3<f32>(0., 0., 0.);
	ro.y = sin(t*.2)*15.+15.;
	ro.x = sin(t*.5)*5.;
	ro.z = t*5.;
	let rd = cam(uv, p, vec3<f32>(0,1,0), p + vec3<f32>(0,0,1));
	var s = 1.;
	let h = .15;
	var td = 0.;
	var d=1.;
	var dd=0.;
	var w = 0.;
    let v = 0.03;
    for(var i=0.;i<200.;i = i + 1.)
	{      
		if(s<0.01||d>500.||td>.95) { break; }
		if (s > 0.001) {
			s = map(p) * v;
		} else {
			s = map(p) * .2;
		}
        //s = map(p) * (s>0.001?v:.2);
		if (s < h)
		{
			w = (1.-td) * (h-s)*i/200.;
			f += w;
			td += w;
		}
		dd += 0.012;
		td += 0.005;
		s = max(s, 0.05);
		d+=s;	
		p = ro+rd*d;	
   	}
	var rgbn = mix( f.rgb, vec3<f32>(0,0.15,0.52), 1.0 - exp( -0.001*d*d) )/dd; // fog
	
	// vigneting from iq Shader Mike : https://www.shadertoy.com/view/MsXGWr
    let q = input.uv/si;
    rgbn *= 0.5 + 0.5*pow( 16.0*q.x*q.y*(1.0-q.x)*(1.0-q.y), 0.25 );
	f.r = rgbn.r;
	f.g = rgbn.g;
	f.b = rgbn.b;
	return f;
}