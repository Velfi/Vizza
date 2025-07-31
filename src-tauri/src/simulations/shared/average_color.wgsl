// Shared compute shader to calculate average color of display texture
// Used for background color when zoomed out in infinite rendering

@group(0) @binding(0)
var display_tex: texture_2d<f32>;

@group(0) @binding(1)
var<storage, read_write> average_color: array<atomic<u32>>; // [r, g, b, a] - atomic u32 for parallel reduction

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let tex_width = u32(textureDimensions(display_tex).x);
    let tex_height = u32(textureDimensions(display_tex).y);
    
    if (id.x >= tex_width || id.y >= tex_height) {
        return;
    }
    
    // Sample the color at this pixel
    let color = textureLoad(display_tex, vec2<i32>(i32(id.x), i32(id.y)), 0);
    
    // Convert float colors to fixed-point integers (multiply by 255 for 8-bit precision)
    let r_int = u32(color.r * 255.0);
    let g_int = u32(color.g * 255.0);
    let b_int = u32(color.b * 255.0);
    let a_int = u32(color.a * 255.0);
    
    // Add to running sum using atomic operations
    atomicAdd(&average_color[0], r_int);
    atomicAdd(&average_color[1], g_int);
    atomicAdd(&average_color[2], b_int);
    atomicAdd(&average_color[3], a_int);
} 