struct VertexInput {
    [[location(0)]] pos: vec3<f32>;
    [[location(1)]] norm: vec3<f32>;
    [[location(2)]] albedo: vec4<f32>;
};

struct VertexOutput {
    [[builtin(position)]] pos: vec4<f32>;
    [[location(0)]] normal: vec3<f32>;
    [[location(1)]] depth: f32;
    [[location(2)]] albedo: vec4<f32>;
};

[[block]]
struct Uniforms {
    camera_iview: mat4x4<f32>;
    camera_proj: mat4x4<f32>;
    transform: mat4x4<f32>;
};

[[group(0), binding(0)]]
var uniforms: Uniforms;

[[stage(vertex)]]
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    out.pos = uniforms.camera_proj * uniforms.camera_iview * uniforms.transform * vec4<f32>(in.pos, 1.0);
    out.normal = (uniforms.transform * vec4<f32>(in.norm, 0.0)).xyz;
    out.depth = out.pos.z / out.pos.w;
    out.albedo = in.albedo;

    return out;
}

struct FragmentOutput {
    [[location(0)]] norm_depth: vec4<f32>;
    [[location(1)]] albedo: vec4<f32>;
};

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    out.norm_depth = vec4<f32>(in.normal, in.pos.z);
    out.albedo = in.albedo;

    return out;
}
