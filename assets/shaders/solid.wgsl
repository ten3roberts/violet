struct VertexInput {
    @location(0) pos: vec3<f32>,
    @builtin(instance_index) instance: u32,
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
}

struct Object {
    world_matrix: mat4x4<f32>,
    color: vec4<f32>,
}

struct Globals {
    viewproj: mat4x4<f32>,
}
    
@group(0) @binding(0)
var<uniform> globals: Globals;

@group(1) @binding(0)
var<storage> objects: array<Object>;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let object = objects[in.instance];
    out.pos = globals.viewproj * object.world_matrix * vec4<f32>(in.pos, 1.0);
    out.color = object.color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
