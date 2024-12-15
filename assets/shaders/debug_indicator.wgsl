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
    @location(2) vertex_pos: vec3<f32>,
    @location(3) scale: vec2<f32>,
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
    let scale = (object.world_matrix * vec4(1.0, 1.0, 0.0, 0.0)).xy;
    out.pos = globals.viewproj * object.world_matrix * vec4<f32>(in.pos, 1.0);
    out.color = object.color;
    out.tex_coord = in.tex_coord;
    out.vertex_pos = in.pos;
    out.scale = scale;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    let nearest_corner = vec2<f32>(select(0.0, in.scale.x, in.vertex_pos.x > 0.5), select(0.0, in.scale.y, in.vertex_pos.y > 0.5));
    let to_nearest = abs(in.vertex_pos.xy * in.scale - nearest_corner);

    // return vec4(length(nearest_corner - in.pos.xy) / 100.0);
    let dist = max(1.0 - length(to_nearest) * 0.1, 0.0);
    // return vec4(0.0, 0.0, dist, 1.0);
    let border_size = 8.0;

    var border = 0.0;

    // if (to_nearest.x < border_size || to_nearest.y < border_size ||
    //     to_nearest.x > in.scale.x - border_size || to_nearest.y > in.scale.y - border_size) {
    //         border = 1.0;
    // }
    if dot(to_nearest, vec2(1.0)) < border_size {
        border = 1.0;
    }

    return in.color * textureSample(fill_image, default_sampler, in.tex_coord) * border;
}
