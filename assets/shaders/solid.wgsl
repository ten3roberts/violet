struct VertexInput {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tex_coord: vec2<f32>,
    @builtin(instance_index) instance: u32,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) frag_pos: vec3<f32>,
    @location(3) frag_scale: vec3<f32>,
    @location(4) corner_radius: f32,
}

struct Object {
    world_matrix: mat4x4<f32>,
    color: vec4<f32>,
    corner_radius: f32,
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

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let object = objects[in.instance];


    let scale = vec3(object.world_matrix[0][0], object.world_matrix[1][1], object.world_matrix[2][2]);

    out.pos = globals.viewproj * object.world_matrix * vec4<f32>(in.pos, 1.0);
    out.color = object.color;
    out.tex_coord = in.tex_coord;
    out.frag_pos = in.pos;
    out.frag_scale = abs(scale);
    out.corner_radius = object.corner_radius;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let width = in.frag_scale.x / 2;
    let height = in.frag_scale.y / 2;

    var alpha = 1f;
    let corner_radius = min(in.corner_radius, min(width, height));
    if corner_radius != 0.0 {
        let midsegment = vec2(width - corner_radius, height - corner_radius);

        let local_pos = ((in.frag_pos.xy - 0.5) * in.frag_scale.xy);

        let corner_pos = midsegment * sign(local_pos);

        let inside_corner = max((vec2(width, height) - corner_radius) - abs(local_pos), vec2(0f));

        let corner_cutout = smoothstep(-1f, 1f, corner_radius - length(local_pos - corner_pos)) + inside_corner.x + inside_corner.y;
        alpha = min(corner_cutout, 1.0);
    }

    return in.color * textureSample(fill_image, default_sampler, in.tex_coord) * vec4(1f, 1f, 1f, alpha);
}
