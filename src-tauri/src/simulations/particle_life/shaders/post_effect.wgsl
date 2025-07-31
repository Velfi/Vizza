struct PostEffectParams {
    brightness: f32,
    contrast: f32,
    saturation: f32,
    gamma: f32,
}

@group(0) @binding(0)
var display_tex: texture_2d<f32>;
@group(0) @binding(1)
var display_sampler: sampler;
@group(0) @binding(2)
var<uniform> params: PostEffectParams;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0)
    );
    
    let uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0)
    );
    
    let pos = positions[vertex_index];
    let uv = uvs[vertex_index];
    
    return VertexOutput(
        vec4<f32>(pos, 0.0, 1.0),
        uv
    );
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let base_color = textureSample(display_tex, display_sampler, uv);
    
    // Apply brightness
    var color = base_color.rgb * params.brightness;
    
    // Apply contrast
    color = (color - 0.5) * params.contrast + 0.5;
    
    // Apply saturation
    let luminance = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    color = mix(vec3<f32>(luminance), color, params.saturation);
    
    // Apply gamma
    color = pow(color, vec3<f32>(1.0 / params.gamma));
    
    return vec4<f32>(color, base_color.a);
} 