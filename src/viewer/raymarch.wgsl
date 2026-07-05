struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) screen_uv: vec2<f32>,
};

struct CameraUniform {
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
};

struct ViewSettings {
    slice_depth: f32,
    window_center: f32,
    window_width: f32,
    padding: f32,
};

@group(0) @binding(0) var t_volume: texture_3d<u32>;
@group(0) @binding(1) var<uniform> camera: CameraUniform;
@group(0) @binding(2) var<uniform> settings: ViewSettings;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Extra big triangle same as in 2d
    let x = f32(i32(in.vertex_index & 1u) << 2u) - 1.0;
    let y = f32(i32(in.vertex_index & 2u) << 1u) - 1.0;
    
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.screen_uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return out;
}

fn ray_box_intersection(ray_orig: vec3<f32>, ray_dir: vec3<f32>, box_min: vec3<f32>, box_max: vec3<f32>, t_near: ptr<function, f32>, t_far: ptr<function, f32>) -> bool {
    let inv_dir = 1.0 / ray_dir;
    let t_bot = inv_dir * (box_min - ray_orig);
    let t_top = inv_dir * (box_max - ray_orig);
    
    let t_min = min(t_bot, t_top);
    let t_max = max(t_bot, t_top);
    
    let t0 = max(t_min.x, max(t_min.y, t_min.z));
    let t1 = min(t_max.x, min(t_max.y, t_max.z));
    
    *t_near = t0;
    *t_far = t1;
    
    return t0 <= t1 && t1 > 0.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 1. Convert UV coordinates back to 3D ray direction
    let ndc_target = vec4<f32>(in.screen_uv.x * 2.0 - 1.0, (1.0 - in.screen_uv.y) * 2.0 - 1.0, 0.0, 1.0);
    let world_target_unhomogenized = camera.inv_view_proj * ndc_target;
    let world_target = world_target_unhomogenized.xyz / world_target_unhomogenized.w;
    
    let ray_start = camera.camera_pos.xyz;
    let ray_dir = normalize(world_target - ray_start);
    
    // Define virtual data cube boundaries in space
    let box_min = vec3<f32>(-0.5, -0.5, -0.5);
    let box_max = vec3<f32>(0.5, 0.5, 0.5);
    
    var t_near: f32 = 0.0;
    var t_far: f32 = 0.0;
    
    // 2. Check if the camera ray hits our 3D data
    if (!ray_box_intersection(ray_start, ray_dir, box_min, box_max, &t_near, &t_far)) {
        discard;
    }
    
    t_near = max(t_near, 0.0);
    
    // 3. Initialize raymarching parameters
    var accum_color = vec3<f32>(0.0);
    var accum_alpha = 0.0;

    // What is a good step size? 
    let num_steps = 150;
    let step_size = (t_far - t_near) / f32(num_steps);
    
    var current_ray_time = t_near;
    let tex_size = vec3<f32>(textureDimensions(t_volume));

    // 4. Trace the ray through the body
    for (var i = 0; i < num_steps; i = i + 1) {
        if (accum_alpha >= 0.95) { break; } // Performance shortcut: Stop if opaque
        
        let world_pos = ray_start + ray_dir * current_ray_time;
        
        let local_uv = world_pos + vec3<f32>(0.5);
        let coords = vec3<i32>(local_uv * tex_size);
        
        if (any(local_uv < vec3<f32>(0.0)) || any(local_uv > vec3<f32>(1.0))) {
            current_ray_time = current_ray_time + step_size;
            continue;
        }

        // 5. Convert back to Hounsfield Units
        let raw_val_u32 = textureLoad(t_volume, coords, 0).r;
        let raw_val = f32(raw_val_u32) - 1024.0;
        
        // Window windowing calculations
        let half_width = settings.window_width / 2.0;
        let lower_bound = settings.window_center - half_width;
        let intensity = clamp((raw_val - lower_bound) / settings.window_width, 0.0, 1.0);

        // 6. Alpha-blend accumulated voxels together
        if (intensity > 0.05) {
            // Scale opacity down so we can see inside soft structures
            let sample_alpha = intensity * 0.03; 
            
            accum_color = accum_color + (vec3<f32>(intensity) * sample_alpha * (1.0 - accum_alpha));
            accum_alpha = accum_alpha + sample_alpha * (1.0 - accum_alpha);
        }
        
        current_ray_time = current_ray_time + step_size;
    }

    return vec4<f32>(accum_color, accum_alpha);
}