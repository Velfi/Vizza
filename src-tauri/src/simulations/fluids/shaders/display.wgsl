struct CameraUniform {
    transform_matrix: mat4x4<f32>;
    position: vec2<f32>;
    zoom: f32;
    aspect_ratio: f32;
}

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var display_tex: texture_2d<f32>;
@group(0) @binding(1) var display_sampler: sampler;
@group(1) @binding(0) var<uniform> camera: CameraUniform;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );
    var uv = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
    );
    var o: VSOut;
    o.pos = camera.transform_matrix * vec4<f32>(pos[vi], 0.0, 1.0);
    o.uv = uv[vi];
    return o;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    return textureSample(display_tex, display_sampler, in.uv);
}

