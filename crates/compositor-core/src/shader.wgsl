struct Uniforms {
    transform: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var t_diffuse: texture_2d<f32>;

@group(0) @binding(2)
var s_diffuse: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    var pos_array = array<vec2<f32>, 6>(
        vec2<f32>(-1.0,  1.0), // top-left
        vec2<f32>(-1.0, -1.0), // bottom-left
        vec2<f32>( 1.0, -1.0), // bottom-right
        vec2<f32>(-1.0,  1.0), // top-left
        vec2<f32>( 1.0, -1.0), // bottom-right
        vec2<f32>( 1.0,  1.0)  // top-right
    );

    var uv_array = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0)
    );

    let p = pos_array[in_vertex_index];
    let u = uv_array[in_vertex_index];
    
    out.clip_position = uniforms.transform * vec4<f32>(p.x, p.y, 0.0, 1.0);
    out.tex_coords = u;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Return the sampled texture, or black if out of bounds (handled by clamp_to_edge but we scaled the pos)
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}
