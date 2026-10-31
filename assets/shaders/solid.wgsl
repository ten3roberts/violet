struct VertexInput {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tex_coord: vec2<f32>,
    @builtin(instance_index) instance: u32,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    // Position in the shape's unit square; outside [0, 1] where the quad grows to fit the shadow
    @location(0) unit_pos: vec2<f32>,
    @location(1) @interpolate(flat) instance: u32,
}

struct Object {
    world_matrix: mat4x4<f32>,
    color: vec4<f32>,
    border_color: vec4<f32>,
    shadow_color: vec4<f32>,
    inset_shadow_color: vec4<f32>,
    // offset.xy, blur, spread
    shadow: vec4<f32>,
    inset_shadow: vec4<f32>,
    corner_radius: f32,
    border_width: f32,
    _padding: vec2<f32>,
}

struct Globals {
    viewproj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> globals: Globals;

@group(1) @binding(0)
var<uniform> objects: array<Object, 32>;

@group(2) @binding(0)
var default_sampler: sampler;

@group(2) @binding(1)
var fill_image: texture_2d<f32>;

fn object_size(object: Object) -> vec2<f32> {
    return abs(vec2(object.world_matrix[0][0], object.world_matrix[1][1]));
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let object = objects[in.instance];
    let size = max(object_size(object), vec2(1e-4));

    // Grow the quad to hold the outer shadow, one pixel more for anti-aliasing
    var pad = vec2(0.0);
    if object.shadow_color.a > 0.0 {
        let shadow = object.shadow;
        pad = abs(shadow.xy) + shadow.z + max(shadow.w, 0.0) + 1.0;
    }

    let unit_pos = in.pos.xy + (in.pos.xy * 2.0 - 1.0) * pad / size;

    out.pos = globals.viewproj * object.world_matrix * vec4<f32>(unit_pos, in.pos.z, 1.0);
    out.unit_pos = unit_pos;
    out.instance = in.instance;

    return out;
}

/// Signed distance to a rounded rectangle centred on the origin
fn sd_rounded_rect(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + radius;
    return length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

fn erf(x: f32) -> f32 {
    // Winitzki's approximation
    let a = 0.147;
    let x2 = x * x;
    return sign(x) * sqrt(1.0 - exp(-x2 * (1.2732395 + a * x2) / (1.0 + a * x2)));
}

/// Coverage of the shape whose signed distance is `d`, blurred over a CSS blur radius
fn blurred_coverage(d: f32, blur: f32) -> f32 {
    // CSS blurs with a Gaussian of half the blur radius; below half a pixel it is the edge's
    // anti-aliasing
    let sigma = max(blur * 0.5, 0.5);
    return 0.5 - 0.5 * erf(d / (sigma * 1.4142135));
}

/// Places a premultiplied `top` over a premultiplied `bottom`
fn over(top: vec4<f32>, bottom: vec4<f32>) -> vec4<f32> {
    return top + bottom * (1.0 - top.a);
}

fn premultiply(color: vec4<f32>, coverage: f32) -> vec4<f32> {
    let a = color.a * coverage;
    return vec4(color.rgb * a, a);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let object = objects[in.instance];
    let size = object_size(object);
    let half_size = size * 0.5;
    let p = (in.unit_pos - 0.5) * size;

    let radius = clamp(object.corner_radius, 0.0, min(half_size.x, half_size.y));
    let d = sd_rounded_rect(p, half_size, radius);
    let fill_coverage = clamp(0.5 - d, 0.0, 1.0);

    let tex_coord = clamp(in.unit_pos, vec2(0.0), vec2(1.0));
    let fill = object.color * textureSample(fill_image, default_sampler, tex_coord);
    var color = premultiply(fill, fill_coverage);

    if object.inset_shadow_color.a > 0.0 {
        // The shadow is the edge of the shape, offset and shrunk by the spread, cast inwards
        let shadow = object.inset_shadow;
        let hole_half = max(half_size - shadow.w, vec2(0.0));
        let hole_radius = max(radius - shadow.w, 0.0);
        let hole = sd_rounded_rect(p - shadow.xy, hole_half, hole_radius);
        let amount = 1.0 - blurred_coverage(hole, shadow.z);
        color = over(premultiply(object.inset_shadow_color, amount * fill_coverage), color);
    }

    if object.border_width > 0.0 {
        let inner_coverage = clamp(0.5 - (d + object.border_width), 0.0, 1.0);
        color = over(premultiply(object.border_color, fill_coverage - inner_coverage), color);
    }

    if object.shadow_color.a > 0.0 {
        // Drawn only outside the shape, as CSS does, so it never shows through a translucent fill
        let shadow = object.shadow;
        let spread_half = max(half_size + shadow.w, vec2(0.0));
        let spread_radius = max(radius + shadow.w, 0.0);
        let shadow_d = sd_rounded_rect(p - shadow.xy, spread_half, spread_radius);
        let amount = blurred_coverage(shadow_d, shadow.z) * (1.0 - fill_coverage);
        color = over(color, premultiply(object.shadow_color, amount));
    }

    if color.a <= 0.0 {
        discard;
    }

    return vec4(color.rgb / color.a, color.a);
}
