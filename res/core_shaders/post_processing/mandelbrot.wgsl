struct Immediates {
    offset_x: vec2<f32>,
    offset_y: vec2<f32>,
    scale: vec2<f32>,
};
var<immediate> config: Immediates;

struct Uniform {
    time: f32,
    res: vec2<f32> 
};
@group(0) @binding(0)
var<uniform> post_uniform: Uniform;

@group(1) @binding(0)
var scene_tex: texture_2d<f32>;
@group(1) @binding(1)
var scene_sampler: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VsOut {
    var positions = array<vec2<f32>,3>(
        vec2(-1.0,-1.0),
        vec2( 3.0,-1.0),
        vec2(-1.0, 3.0)
    );

    var out: VsOut;

    var p = positions[i];

    out.pos = vec4(p,0.0,1.0);
    out.uv = p * 0.5 + 0.5;
    out.uv.y = 1.0 - out.uv.y;

    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var real = in.uv.x;
    var im = in.uv.y;
    var len = vec2<f32>(post_uniform.res.x, 0.0);
    var dim = vec2<f32>(post_uniform.res.x / 2.0, 0.0);
    var dim_scale = ds_mul(dim, config.scale);

    var out: vec4<f32>;

    var cx = ds_add(ds_mul(vec2<f32>(in.uv.x - 0.5, 0.0), dim_scale), config.offset_x);

    var cy = ds_add(ds_mul(vec2<f32>(in.uv.y - 0.5, 0.0), dim_scale), config.offset_y);
    var zx = vec2<f32>(0.0, 0.0);
    var zy = vec2<f32>(0.0, 0.0);
    var j = 0;

    while zx.x * zx.x + zy.x * zy.x <= 256.0 && j < 100 {
        // 1. Calculate zx^2 and zy^2
        let zx_sqr = ds_sqr(zx); 
        let zy_sqr = ds_sqr(zy);
        
        // 2. Real part: zx^2 - zy^2 + x
        let diff = ds_sub(zx_sqr, zy_sqr);
        let temp = ds_add(diff, cx); 
        
        // 3. Imaginary part: 2 * zx * zy + y
        let zxzy = ds_mul(zx, zy); 
        let two_zxzy = zxzy * 2.0; // Exact scaling for powers of two is safe
        zy = ds_add(two_zxzy, cy); 
        
        // 4. Update real part and iterate
        zx = temp; 
        j += 1;
    }
    
    if zx.x * zx.x + zy.x * zy.x > 256.0 {
        var t = exp2(2.0 * log2(sin(f32(j) / 20.0)));
        var col = rgb(t);
        out = vec4<f32>(col, 1.0);
    } else {
        out = vec4<f32>(0.0,0.0,0.0,1.0);
    }

    return out;
}


fn rgb(t: f32) -> vec3<f32> {
    var h = 360.0 * t;
    var x = 1.0 - abs(((h / 60.0) % 2.0) - 1.0);
    var rgb = vec3<f32>(0.,0.,0.);
    if h < 60.0 {
        rgb = vec3<f32>(1.0, x, 0.0);
    } else if h < 120.0 {
        rgb = vec3<f32>(x, 1.0, 0.0);
    } else if h < 180.0 {
        rgb = vec3<f32>(0.0, 1.0, x);
    } else if h < 240.0 {
        rgb = vec3<f32>(0.0, x, 1.0);
    } else if h < 300.0 {
        rgb = vec3<f32>(x, 0.0, 1.0);
    } else {
        rgb = vec3<f32>(1.0, 0.0, x);
    }
    return vec3<f32>(rgb[0], rgb[1], rgb[2]);
}



fn ds_normalize(a: vec2<f32>) -> vec2<f32> {
    let s = a.x + a.y; let e = a.y - (s - a.x); return vec2<f32>(s, e);
}
fn ds_add(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    let s = a.x + b.x; let v = s - a.x; let e = (a.x - (s - v)) + (b.x - v) + a.y + b.y;
    return ds_normalize(vec2<f32>(s, e));
}
fn ds_sub(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    let s = a.x - b.x; let v = s - a.x; let e = (a.x - (s - v)) - (b.x + v) + a.y - b.y;
    return ds_normalize(vec2<f32>(s, e));
}
fn ds_mul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    let hi = a.x * b.x; let lo = fma(a.x, b.x, -hi) + a.x * b.y + a.y * b.x;
    return ds_normalize(vec2<f32>(hi, lo));
}
fn ds_sqr(a: vec2<f32>) -> vec2<f32> {
    let hi = a.x * a.x; let lo = fma(a.x, a.x, -hi) + (a.x * a.y) * 2.0;
    return ds_normalize(vec2<f32>(hi, lo));
}
