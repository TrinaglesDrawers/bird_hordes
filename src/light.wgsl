
[[group(0), binding(0)]]
var norm_depths: texture_2d<f32>;

[[group(0), binding(1)]]
var albedos: texture_2d<f32>;

struct DirLight {
    dir: vec3<f32>;
    color: vec3<f32>;
};

struct PointLight {
    point: vec3<f32>;
    color: vec3<f32>;
};

[[block]]
struct Uniforms {
    camera_view: mat4x4<f32>;
    camera_iproj: mat4x4<f32>;

    directional: DirLight;
    point: [[stride(32)]] array<PointLight, 4>;
};

[[group(0), binding(2)]]
var uniforms: Uniforms;

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] world_xy: vec2<f32>;
};

[[stage(vertex)]]
fn vs_light([[builtin(vertex_index)]] vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    let x: f32 = f32((vertex_index & u32(1)) << u32(2));
    let y: f32 = f32((vertex_index & u32(2)) << u32(1));

    out.world_xy = vec2<f32>(x - 1.0, y - 1.0);
    out.position = vec4<f32>(out.world_xy, 0.0, 1.0);

    return out;
}

[[stage(fragment)]]
fn fs_light(in: VertexOutput) -> [[location(0)]] vec4<f32> {

    let norm_depth: vec4<f32> = textureLoad(norm_depths, vec2<i32>(in.position.xy), 0);
    let normal: vec3<f32> = norm_depth.xyz;
    let depth: f32 = norm_depth.w;

    let albedo: vec3<f32> = textureLoad(albedos, vec2<i32>(in.position.xy), 0).xyz;

    let world: vec3<f32> = (uniforms.camera_view * uniforms.camera_iproj * vec4<f32>(in.world_xy, depth, 1.0)).xyz;

    var sum: vec3<f32> = vec3<f32>(0.0);

    let d: f32 = -dot(uniforms.directional.dir, normal);
    if (d > 0.0)
    {
        sum = sum + uniforms.directional.color * d;
    }

    return vec4<f32>(sum * albedo, 1.0);
}
