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

// Fractal Generation - using the genome
fn sdFractalChannel(pos: vec3<f32>, genome_genes: array<vec4<f32>, 4>) -> f32 {
    var p = pos;
    var d = sdBox(p, vec3<f32>(20.0, 20.0, 20.0)); 

    let iterations = 4;
    
    // Unpack genes
    let g0 = get_gene(0, genome_genes);
    let g1 = get_gene(1, genome_genes);
    let g2 = get_gene(2, genome_genes);
    let g3 = get_gene(3, genome_genes);
    let g4 = get_gene(4, genome_genes);

    let scale = g0 * 2.0 + 1.0; 
    let offset = vec3<f32>(g1, g2, g3) * 5.0; 
    let rot_angle = g4 * 3.14159 * 2.0; 

    for (var i = 0; i < iterations; i++) {
        p = abs(p + offset) - offset; 
        p = rotY(p, rot_angle);       
        p = p * scale;                
        d = min(d, sdBox(p, vec3<f32>(1.0)) / pow(scale, f32(i + 1)));
    }
    
    return -d; 
}

fn map_geometry(p: vec3<f32>, genome_genes: array<vec4<f32>, 4>) -> f32 {
    return sdFractalChannel(p, genome_genes);
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