struct ModelUniform {
    world: mat4x4<f32>,
    inv_world: mat4x4<f32>,
    world_view_proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    camera_dir: vec4<f32>,
    target_dimensions: vec4<f32>,
    time_colorpow_: vec4<f32>,
    custom_data_1: vec4<f32>
};

@group(1) @binding(0)
var<uniform> model_uniform: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) inv_light_1: vec3<f32>,
    @location(3) inv_light_2: vec3<f32>,
    @location(4) inv_light_3: vec3<f32>
}

@vertex
fn vs_main(
    model: VertexInput
) -> VertexOutput {
    var out: VertexOutput;

    out.tex_coords = model.tex_coords;

    var pos: vec3<f32> = model.position.xyz;
    var normal = vec4<f32>(model.normal.xyz, 0.0);
    out.normal = (model_uniform.inv_world * normal).xyz;

    out.clip_position = model_uniform.world_view_proj * vec4<f32>(pos.xyz, 1.0);
    out.inv_light_1 = (model_uniform.inv_world * vec4<f32>(1.0, 1.0, 1.0, 0.0)).xyz;
    out.inv_light_2 = (model_uniform.inv_world * vec4<f32>(-1.0, 1.0, 1.0, 0.0)).xyz;
    out.inv_light_3 = (model_uniform.inv_world * vec4<f32>(0.0, 1.0, 0.0, 0.0)).xyz;

//out.clip_position.z = 0.5;
//out.clip_position.w = 0.5;

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
    var uv : vec2<f32> = in.tex_coords;

    var scroll_uv1 = in.tex_coords * vec2<f32>(1.3, 1.3) + vec2<f32>(0.0, 3.0) * model_uniform.time_colorpow_.x;
    var color1 = textureSample(t_diffuse, s_diffuse, scroll_uv1).x * vec3<f32>(0.4, 0.0, 0.0);

    var scroll_uv2 = in.tex_coords * vec2<f32>(2.0, 2.0) + vec2<f32>(0.0, 0.1) * model_uniform.time_colorpow_.x;
    var color2 = textureSample(t_diffuse, s_diffuse, scroll_uv2).y * vec3<f32>(0.6, 0.3, 2.0);

    var scroll_uv3 = in.tex_coords * vec2<f32>(0.55, 0.55) + vec2<f32>(0.0, 0.5) * model_uniform.time_colorpow_.x;
    var color3 = textureSample(t_diffuse, s_diffuse, scroll_uv3).y * vec3<f32>(1.0, 1.0, 0.0);

    let albedo = (color1 + color2 + color3) * 0.5;

    outColor.x = albedo.x;
    outColor.y = albedo.y;
    outColor.z = albedo.z;
    outColor.w = max(max(albedo.x, albedo.y), albedo.z);

    return outColor;
}