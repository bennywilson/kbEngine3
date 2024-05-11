struct ModelUniform {
    world_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    camera_dir: vec4<f32>,
    sun_color: vec4<f32>,
};
@group(0) @binding(0)
var<uniform> uniform_buffer: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct InstanceInput {
    @location(10) pos_scale: vec4<f32>,

}
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(in_vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    var pos = in_vertex.position.xyz;
    pos.x = ((pos.x - instance.pos_scale.x) * instance.pos_scale.z) + instance.pos_scale.x;
    pos.y = ((pos.y - instance.pos_scale.y) * instance.pos_scale.z) + instance.pos_scale.y;

    out.clip_position = vec4<f32>(pos, 1.0);

    out.tex_coords = in_vertex.tex_coords;
    return out;
}

/**
 *  Fragment Shader
 */

@group(1) @binding(0)
var mask_texture: texture_2d<f32>;
@group(1) @binding(1)
var mask_sampler: sampler;
@group(1) @binding(2)
var flare_texture: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var mask_rgba = textureSample(mask_texture, mask_sampler, in.tex_coords.xy);
    var flare_uv = mask_rgba.xy;
    var mask = mask_rgba.z;

    var out = uniform_buffer.sun_color.xyzw * mask * textureSample(flare_texture, mask_sampler, flare_uv.xy);
    return out;
}