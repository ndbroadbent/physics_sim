// Render Shader for 3D Raymarching Visualization

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    _pad1: f32, // Align to 16 bytes
    camera_pos: vec3<f32>,
    _pad2: f32, // Align vec3 to 16 bytes
    camera_target: vec3<f32>,
    _pad3: f32, // Align vec3 to 16 bytes
    camera_up: vec3<f32>,
    _pad4: f32, // Align vec3 to 16 bytes
    genome_params: array<vec4<f32>, 4>, 
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

// Helper to extract genes from the packed vec4 array
fn get_gene(index: i32, genes: array<vec4<f32>, 4>) -> f32 {
    let vec_idx = index / 4;
    let comp_idx = index % 4;
    let v = genes[vec_idx];
    if (comp_idx == 0) { return v.x; }
    if (comp_idx == 1) { return v.y; }
    if (comp_idx == 2) { return v.z; }
    return v.w;
}

// Rotates a point p by angle 'a' around Y axis
fn rotY(p: vec3<f32>, a: f32) -> vec3<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec3<f32>(c * p.x + s * p.z, p.y, -s * p.x + c * p.z);
}

// --- Primitives ---

// Box
fn sdBox(p: vec3<f32>, b: vec3<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}

// Sphere
fn sdSphere(p: vec3<f32>, s: f32) -> f32 {
    return length(p) - s;
}

// Plane (Floor)
fn sdPlane(p: vec3<f32>, height: f32) -> f32 {
    return p.y - height;
}

// --- Fractal Generation ---
// Not used currently, but kept for reference
fn sdFractalChannel(pos: vec3<f32>, genes_array: array<vec4<f32>, 4>) -> f32 {
    var p = pos;
    var d = sdBox(p, vec3<f32>(10.0, 10.0, 10.0)); // Smaller bounding box

    let iterations = 8; // Increased iterations for more detail
    
    // Unpack genes using helper
    let scale = get_gene(0, genes_array) * 2.0 + 1.0; 
    let offset = vec3<f32>(get_gene(1, genes_array), get_gene(2, genes_array), get_gene(3, genes_array)) * 5.0; 
    let rot_angle = get_gene(4, genes_array) * 3.14159 * 2.0; 

    for (var i = 0; i < iterations; i++) {
        p = abs(p + offset) - offset; 
        p = rotY(p, rot_angle);       
        p = p * scale;                
        // Subtract box to create channels
        let box_dist = sdBox(p, vec3<f32>(1.0));
        let scale_factor = pow(scale, f32(i + 1));
        d = max(d, -box_dist / scale_factor);
    }
    
    return d; 
}


// Main evaluation function
fn map_geometry(p: vec3<f32>, genes_array: array<vec4<f32>, 4>) -> f32 {
    let fractal_dist = sdFractalChannel(p - vec3<f32>(0.0, 10.0, 0.0), genes_array); // Lift fractal up by 10
    let floor_dist = sdPlane(p, -10.0); // Floor at y = -10
    return min(fractal_dist, floor_dist);
}

// --- Raymarching ---

fn get_normal(p: vec3<f32>, genome_genes: array<vec4<f32>, 4>) -> vec3<f32> {
    let eps = 0.001;
    let normal_x = map_geometry(p + vec3<f32>(eps, 0.0, 0.0), genome_genes) - map_geometry(p - vec3<f32>(eps, 0.0, 0.0), genome_genes);
    let normal_y = map_geometry(p + vec3<f32>(0.0, eps, 0.0), genome_genes) - map_geometry(p - vec3<f32>(0.0, eps, 0.0), genome_genes);
    let normal_z = map_geometry(p + vec3<f32>(0.0, 0.0, eps), genome_genes) - map_geometry(p - vec3<f32>(0.0, 0.0, eps), genome_genes);
    return normalize(vec3<f32>(normal_x, normal_y, normal_z));
}

fn ray_march(ro: vec3<f32>, rd: vec3<f32>, genome_genes: array<vec4<f32>, 4>) -> f32 {
    var total_dist = 0.0;
    for (var i = 0; i < 100; i++) { // Max steps
        let p = ro + rd * total_dist;
        let dist = map_geometry(p, genome_genes);
        if (dist < 0.001) { return total_dist; } // Hit!
        total_dist += dist;
        if (total_dist > 100.0) { break; } // Max render distance
    }
    return -1.0; // No hit
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let pos = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );

    let uv_coords = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 0.0),
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos[vertex_index], 0.0, 1.0);
    out.uv = uv_coords[vertex_index];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv * 2.0 - 1.0; 
    let ar = uniforms.resolution.x / uniforms.resolution.y;

    // Camera setup
    let ro = uniforms.camera_pos;
    let cam_target = uniforms.camera_target;
    let up = uniforms.camera_up;

    let cz = normalize(cam_target - ro);
    let cx = normalize(cross(cz, up));
    let cy = normalize(cross(cx, cz));

    let rd = normalize(cx * uv.x * ar + cy * uv.y + cz);

    // Raymarch
    let t = ray_march(ro, rd, uniforms.genome_params);

    if (t > 0.0) {
        let p = ro + rd * t;
        let normal = get_normal(p, uniforms.genome_params);
        let light_dir = normalize(vec3<f32>(0.5, 0.8, -0.5)); // Sunlight from top-right
        
        // Basic Lambertian diffuse
        let diffuse = max(dot(normal, light_dir), 0.0);
        
        // Shadow ray (simple hard shadow)
        var shadow = 1.0;
        if (ray_march(p + normal * 0.01, light_dir, uniforms.genome_params) > 0.0) {
            shadow = 0.3; // In shadow
        }

        // Material Color
        var material_color = vec3<f32>(0.7, 0.7, 0.7); // Grey default
        if (p.y < -9.9) { 
            // Floor pattern (checkerboard)
            let f = floor(p.x * 0.5) + floor(p.z * 0.5);
            if ((f % 2.0) == 0.0) {
                material_color = vec3<f32>(0.3, 0.3, 0.3); // Dark tile
            } else {
                material_color = vec3<f32>(0.4, 0.4, 0.4); // Light tile
            }
        } else {
            // Fractal color (Genetic Nano-Machine Blue)
            material_color = vec3<f32>(0.2, 0.6, 0.9);
        }

        let color = material_color * diffuse * shadow + vec3<f32>(0.05); // Ambient
        return vec4<f32>(color, 1.0);
    } else {
        // Sky Gradient
        let t_sky = 0.5 * (rd.y + 1.0);
        let sky_color = mix(vec3<f32>(0.6, 0.7, 0.8), vec3<f32>(0.1, 0.2, 0.4), t_sky); // Horizon to Zenith
        return vec4<f32>(sky_color, 1.0);
    }
}
