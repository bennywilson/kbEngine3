// Vertex shader
struct SpriteUniform {
    target_dimensions: vec4<f32>,
    time_colorpow_: vec4<f32>
};
@group(1) @binding(0)
var<uniform> sprite_uniform: SpriteUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct InstanceInput {
    @location(10) pos_scale: vec4<f32>,
    @location(11) uv_scale_bias: vec4<f32>,
    @location(12) instance_data: vec4<f32>
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    out.tex_coords = (model.tex_coords * instance.uv_scale_bias.xy) + instance.uv_scale_bias.zw;

    var pos: vec3<f32> = model.position.xyz;
    pos *= vec3<f32>(instance.pos_scale.zw, 1.0);
    pos += vec3<f32>(instance.pos_scale.xy, 1.0);
    pos.x *= sprite_uniform.target_dimensions.z;

    out.clip_position = vec4<f32>(pos.xyz, 1.0);

    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;
@group(0) @binding(2)
var t_noise: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var outColor: vec4<f32>;
    var uv : vec2<f32>; 
    uv = in.tex_coords;

    outColor = textureSample(t_diffuse, s_diffuse, uv);
    if (outColor.a < 0.5) {
        discard;
    }

    outColor.r = pow(outColor.r, sprite_uniform.time_colorpow_.y);
    outColor.g = pow(outColor.g, sprite_uniform.time_colorpow_.y);
    outColor.b = pow(outColor.b, sprite_uniform.time_colorpow_.y);

    return outColor;
}