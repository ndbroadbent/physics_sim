// Render Shader for 3D Raymarching Visualization

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    // Padding in Rust struct aligns these, but in WGSL explicit alignment rules apply.
    // Rust: res(8) + time(4) + pad(4) = 16.
    // Cam pos(12) + target(12) + up(12) + pad(12??)
    // Let's check Rust layout:
    // res: [f32; 2] (8 bytes)
    // time: f32 (4 bytes)
    // -- gap 4 bytes -- (implicit in C repr if next field is vec3/16-byte aligned? No, f32 array is 4-byte aligned)
    // cam_pos: [f32; 3] (12 bytes)
    // cam_target: [f32; 3] (12 bytes)
    // cam_up: [f32; 3] (12 bytes)
    // genome: [f32; 16] (64 bytes)
    // padding: [f32; 2] (8 bytes)
    
    // To be safe, let's use vec4s for everything in WGSL to match 16-byte chunks if possible, 
    // OR just fix the array alignment.
    
    // Simplest Fix: Treat genome as vec4 array.
    camera_pos: vec3<f32>,
    camera_target: vec3<f32>,
    camera_up: vec3<f32>,
    
    // We need to be careful with alignment between Rust and WGSL.
    // Uniforms are std140 layout.
    // vec3 is 16-byte aligned/strided in std140.
    // So Rust [f32; 3] needs padding to 16 bytes if mapped to vec3.
    // But our Rust struct is `repr(C)`.
    
    // Let's assume the Rust struct is tightly packed floats.
    // WGSL `uniform` buffer expects specific alignment.
    // Ideally, we should update Rust struct to use `[f32; 4]` for positions.
    
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

// Box
fn sdBox(p: vec3<f32>, b: vec3<f32>) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
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

// --- New: Sawtooth Ratchet Primitive ---
// Matches genetic_sim.wgsl logic
fn sdSawtoothRatchet(p_in: vec3<f32>, genes_array: array<vec4<f32>, 4>) -> f32 {
    var p = p_in;
    // p.x = p.x % 5.0; // Repeat every 5 units in X. WGSL mod is different? 
    // Use fract for repetition: 
    // x = fract(x / 5.0) * 5.0;
    // But standard mod should work? Let's implement manual repeat.
    let repeat = 5.0;
    p.x = p.x - repeat * floor(p.x / repeat);
    
    let tooth_height = get_gene(0, genes_array) * 4.0 + 1.0; // Gene 0: height (1 to 5)
    let tooth_angle = get_gene(1, genes_array) * 0.5 + 0.1; // Gene 1: angle (small bias)
    let wall_thickness = 0.5;

    // A single V-shape.
    // Shift p to make the origin at the tip of the V
    p.x -= 2.5;
    p.y -= tooth_height; 

    // Rotate the space to align with one side of the V
    // let a = atan2(p.y, p.x); // Unused
    // let l = length(p.xy); // Unused
    
    // Define the V-shape using two planes
    // Angle 1
    let d1 = dot(p.xy, vec2<f32>(cos(tooth_angle), sin(tooth_angle)));
    // Angle 2 (negative of angle 1, to make the V)
    let d2 = dot(p.xy, vec2<f32>(cos(-tooth_angle), sin(-tooth_angle)));

    // Combined shape
    let d_v = max(d1, d2) - wall_thickness; // Extrude into a V
    
    // Subtract a flat base
    let base = p.y + tooth_height; // Distance to the floor
    
    return max(d_v, -base); // The shape is the V, constrained by the floor
}

// Main evaluation function
fn map_geometry(p: vec3<f32>, genes_array: array<vec4<f32>, 4>) -> f32 {
    // Switch back to fractal for interesting visuals
    return sdFractalChannel(p, genes_array);
    // return sdSawtoothRatchet(p, genes_array);
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
        let light_dir = normalize(vec3<f32>(0.5, 0.5, -1.0)); // Simple light
        let diffuse = max(dot(normal, light_dir), 0.0);
        let color = vec3<f32>(0.2, 0.5, 0.8) * diffuse + vec3<f32>(0.1); // Blue-ish shaded
        return vec4<f32>(color, 1.0);
    } else {
        return vec4<f32>(0.1, 0.1, 0.2, 1.0); // Background color
    }
}