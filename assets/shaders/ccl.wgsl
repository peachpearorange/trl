struct Params {
    size: vec2<u32>,
    seed: u32,
    _pad: u32,
    world_offset: vec2<i32>,
    _pad2: vec2<i32>,
};

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> parents: array<atomic<u32>>;
@group(0) @binding(2) var dst_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var<uniform> params: Params;

fn pixel_id(xy: vec2<u32>) -> u32 { return xy.y * params.size.x + xy.x; }

fn channel_u8(x: f32) -> u32 { return u32(clamp(floor(x * 255.0 + 0.5), 0.0, 255.0)); }

fn pack_rgba(c: vec4<f32>) -> u32 {
    return channel_u8(c.r) | (channel_u8(c.g) << 8u) | (channel_u8(c.b) << 16u) | (channel_u8(c.a) << 24u);
}

fn is_skip(c: vec4<f32>) -> bool { return channel_u8(c.a) == 0u; }

fn same_color(a: vec4<f32>, b: vec4<f32>) -> bool {
    if is_skip(a) || is_skip(b) { return false; }
    return pack_rgba(a) == pack_rgba(b);
}

fn hash32(v: u32) -> u32 {
    var x = v;
    x ^= x >> 16u; x = x * 0x7feb352du;
    x ^= x >> 15u; x = x * 0x846ca68bu;
    x ^= x >> 16u;
    return x;
}

fn find_root(id: u32) -> u32 {
    var x = id;
    var result = id;
    loop {
        let p = atomicLoad(&parents[x]);
        let gp = atomicLoad(&parents[p]);
        if p == gp { result = p; break; }
        _ = atomicMin(&parents[x], gp);
        x = gp;
    }
    return result;
}

fn union_ids(a: u32, b: u32) {
    loop {
        let ra = find_root(a);
        let rb = find_root(b);
        if ra == rb { break; }
        let hi = max(ra, rb);
        let lo = min(ra, rb);
        let result = atomicCompareExchangeWeak(&parents[hi], hi, lo);
        if result.exchanged { break; }
    }
}

fn rgb_to_hsv(c: vec3<f32>) -> vec3<f32> {
    let k = vec4<f32>(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = mix(vec4<f32>(c.bg, k.wz), vec4<f32>(c.gb, k.xy), step(c.b, c.g));
    let q = mix(vec4<f32>(p.xyw, c.r), vec4<f32>(c.r, p.yzx), step(p.x, c.r));
    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;
    return vec3<f32>(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

fn hsv_to_rgb(c: vec3<f32>) -> vec3<f32> {
    let p = abs(fract(c.xxx + vec3<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - 3.0);
    return c.z * mix(vec3<f32>(1.0), clamp(p - 1.0, vec3<f32>(0.0), vec3<f32>(1.0)), c.y);
}

@compute @workgroup_size(8, 8, 1)
fn init_components(@builtin(global_invocation_id) gid: vec3<u32>) {
    let xy = gid.xy;
    if xy.x >= params.size.x || xy.y >= params.size.y { return; }
    let id = pixel_id(xy);
    atomicStore(&parents[id], id);
}

@compute @workgroup_size(8, 8, 1)
fn union_components(@builtin(global_invocation_id) gid: vec3<u32>) {
    let xy = gid.xy;
    if xy.x >= params.size.x || xy.y >= params.size.y { return; }
    let c = textureLoad(src_tex, vec2<i32>(xy), 0);
    if is_skip(c) { return; }
    let id = pixel_id(xy);
    if xy.x + 1u < params.size.x {
        let nc = textureLoad(src_tex, vec2<i32>(xy + vec2<u32>(1u, 0u)), 0);
        if same_color(c, nc) { union_ids(id, id + 1u); }
    }
    if xy.y + 1u < params.size.y {
        let nc = textureLoad(src_tex, vec2<i32>(xy + vec2<u32>(0u, 1u)), 0);
        if same_color(c, nc) { union_ids(id, id + params.size.x); }
    }
}

@compute @workgroup_size(8, 8, 1)
fn compress_components(@builtin(global_invocation_id) gid: vec3<u32>) {
    let xy = gid.xy;
    if xy.x >= params.size.x || xy.y >= params.size.y { return; }
    let id = pixel_id(xy);
    atomicStore(&parents[id], find_root(id));
}

@compute @workgroup_size(8, 8, 1)
fn recolor_components(@builtin(global_invocation_id) gid: vec3<u32>) {
    let xy = gid.xy;
    if xy.x >= params.size.x || xy.y >= params.size.y { return; }
    let src = textureLoad(src_tex, vec2<i32>(xy), 0);
    if is_skip(src) {
        textureStore(dst_tex, vec2<i32>(xy), src);
        return;
    }
    let root = find_root(pixel_id(xy));
    let rx = i32(root % params.size.x) + params.world_offset.x;
    let ry = i32(root / params.size.x) + params.world_offset.y;
    let world_key = u32(rx) ^ (u32(ry) * 0x9e3779b9u);
    let hr = hash32(world_key ^ params.seed);
    let hg = hash32(hr);
    let hb = hash32(hg);
    let amp = 0.04;
    let dr = (f32(hr & 0xFFFFu) / 65535.0 * 2.0 - 1.0) * amp;
    let dg = (f32(hg & 0xFFFFu) / 65535.0 * 2.0 - 1.0) * amp;
    let db = (f32(hb & 0xFFFFu) / 65535.0 * 2.0 - 1.0) * amp;
    let rgb = clamp(src.rgb + vec3<f32>(dr, dg, db), vec3<f32>(0.0), vec3<f32>(1.0));
    textureStore(dst_tex, vec2<i32>(xy), vec4<f32>(rgb, src.a));
}
