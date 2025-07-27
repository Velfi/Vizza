struct PostEffectParams {
    brightness: f32,
    contrast: f32,
    saturation: f32,
    gamma: f32,
}

@group(0) @binding(0) var<uniform> params: PostEffectParams;
@group(0) @binding(1) var input_texture: texture_2d<f32>;
@group(0) @binding(2) var input_sampler: sampler;

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // Sample the input texture
    let color = textureSample(input_texture, input_sampler, uv);
    
    // Apply brightness
    let bright_color = color.rgb * params.brightness;
    
    // Apply contrast
    let contrast_color = (bright_color - 0.5) * params.contrast + 0.5;
    
    // Apply saturation
    let luminance = dot(contrast_color, vec3<f32>(0.299, 0.587, 0.114));
    let saturated_color = mix(vec3<f32>(luminance), contrast_color, params.saturation);
    
    // Apply gamma correction
    let gamma_corrected = pow(saturated_color, vec3<f32>(1.0 / params.gamma));
    
    return vec4<f32>(gamma_corrected, color.a);
} 