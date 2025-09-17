// GPU-based painting shader for Gray-Scott simulation
// Paints directly onto the texture without clearing it

struct PaintParams {
    mouse_x: f32,
    mouse_y: f32,
    cursor_size: f32,
    cursor_strength: f32,
    mouse_button: u32, // 0 = left (seed), 1 = middle (no effect), 2 = right (erase)
    width: u32,
    height: u32,
    _pad1: u32,
}

@group(0) @binding(0) var<uniform> params: PaintParams;
@group(0) @binding(1) var uvs_texture: texture_storage_2d<rgba32float, read_write>;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);
    
    // Check bounds
    if (x >= i32(params.width) || y >= i32(params.height)) {
        return;
    }
    
    // Convert mouse position to texture coordinates
    let mouse_x = i32(params.mouse_x * f32(params.width));
    let mouse_y = i32(params.mouse_y * f32(params.height));
    
    // Circular brush in pixel space using the smaller texture dimension (no stretching)
    let min_dim = f32(min(params.width, params.height));
    let radius_px = max(params.cursor_size * (min_dim * 0.5), 1.0);

    let dx = f32(x - mouse_x);
    let dy = f32(y - mouse_y);
    let r2 = dx * dx + dy * dy;
    let radius2 = radius_px * radius_px;
    if (r2 > radius2) {
        return;
    }

    // Smooth radial falloff
    let factor = 1.0 - sqrt(r2) / radius_px;
    
    // Skip very small factors to reduce unnecessary updates
    if (factor < 0.01) {
        return;
    }
    
    // Read current UV values
    let current_sample = textureLoad(uvs_texture, vec2<i32>(x, y));
    let current_uv = current_sample.xy;
    
    // Apply painting based on mouse button
    var new_uv = current_uv;
    
    if (params.mouse_button == 0u) {
        // Left mouse button: paint U value based on cursor strength
        let paint_strength = factor * params.cursor_strength;
        new_uv.x = mix(current_uv.x, params.cursor_strength, paint_strength);
        new_uv.y = mix(current_uv.y, 0.8 + 0.2 * factor, paint_strength);
    } else if (params.mouse_button == 2u) {
        // Right mouse button: create voids/erase
        let paint_strength = factor * params.cursor_strength;
        new_uv.x = mix(current_uv.x, 1.0, paint_strength);
        new_uv.y = mix(current_uv.y, 0.0, paint_strength);
    }
    // Middle mouse button (params.mouse_button == 1u): no effect
    
    // Write back to texture
    textureStore(uvs_texture, vec2<i32>(x, y), vec4<f32>(new_uv.x, new_uv.y, 0.0, 0.0));
}
