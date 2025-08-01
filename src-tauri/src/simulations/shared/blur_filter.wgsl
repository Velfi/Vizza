struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct BlurParams {
    radius: f32,
    sigma: f32,
    width: f32,
    height: f32,
}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var<uniform> blur_params: BlurParams;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0)
    );
    
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0)
    );
    
    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = uvs[vertex_index];
    return output;
}

fn gaussian_weight(distance: f32, sigma: f32) -> f32 {
    let sigma_squared = sigma * sigma;
    return exp(-(distance * distance) / (2.0 * sigma_squared)) / sqrt(2.0 * 3.14159 * sigma_squared);
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let radius = blur_params.radius;
    let sigma = blur_params.sigma;
    let width = blur_params.width;
    let height = blur_params.height;
    
    if (radius <= 0.0 || sigma <= 0.0) {
        return textureSample(input_texture, input_sampler, uv);
    }
    
    var result = vec4<f32>(0.0);
    var total_weight = 0.0;
    
    // Horizontal blur
    for (var i = -i32(radius); i <= i32(radius); i++) {
        let offset = vec2<f32>(f32(i) / width, 0.0);
        let sample_uv = uv + offset;
        let weight = gaussian_weight(f32(i), sigma);
        
        result += textureSample(input_texture, input_sampler, sample_uv) * weight;
        total_weight += weight;
    }
    
    result = result / total_weight;
    
    // Vertical blur
    var final_result = vec4<f32>(0.0);
    total_weight = 0.0;
    
    for (var i = -i32(radius); i <= i32(radius); i++) {
        let offset = vec2<f32>(0.0, f32(i) / height);
        let sample_uv = uv + offset;
        let weight = gaussian_weight(f32(i), sigma);
        
        final_result += textureSample(input_texture, input_sampler, sample_uv) * weight;
        total_weight += weight;
    }
    
    return final_result / total_weight;
} 