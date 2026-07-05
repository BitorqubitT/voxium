struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct ViewSettings {
    slice_depth: f32,
    window_center: f32,
    window_width: f32,
    padding: f32,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    let x_u32 = (vertex_index << 1u) & 2u;
    let y_u32 = vertex_index & 2u;
    
    let x = f32(x_u32);
    let y = f32(y_u32);
    
    out.uv = vec2<f32>(x, y);
    
    // Map UV space (0..2) to WebGPU Clip Space (-1..1)
    out.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    
    return out;
}

@group(0) @binding(0) var t_volume: texture_3d<u32>; // Unsigned Integer texture
@group(0) @binding(2) var<uniform> settings: ViewSettings;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_size = textureDimensions(t_volume);
    
    // Clamp the coordinates to [0.0, 0.999] to guarantee we don't round up out of bounds
    let coords = vec3<i32>(
        i32(clamp(in.uv.x, 0.0, 0.999) * f32(tex_size.x)),
        i32(clamp(in.uv.y, 0.0, 0.999) * f32(tex_size.y)),
        i32(clamp(settings.slice_depth, 0.0, 0.999) * f32(tex_size.z))
    );
    
    // Load the raw voxel value directly
    let raw_val_u32 = textureLoad(t_volume, coords, 0).r;
    
    // We 1024 here to get true Hounsfield Units, and slicer values are standard:
    let raw_val = f32(raw_val_u32) - 1024.0;

    let half_width = settings.window_width / 2.0;
    let lower_bound = settings.window_center - half_width;
    
    // Scale the voxel value linearly between lower_bound and upper_bound
    let normalized_bright = (raw_val - lower_bound) / settings.window_width;
    
    let final_bright = clamp(normalized_bright, 0.0, 1.0);
    
    let final_color = vec3<f32>(final_bright, final_bright, final_bright);
    
    return vec4<f32>(final_color, 1.0);
}