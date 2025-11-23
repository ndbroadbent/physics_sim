// FDTD Logic Gate Evolution Shader

// Helper functions

// --- GEOMETRY PLACEHOLDER ---
// The Rust code will inject the SdfOp::to_wgsl() function here as 'map_geometry'.
// It returns distance. d < 0 = Material (High Index), d > 0 = Air (Low Index).
fn map_geometry(p: vec3<f32>) -> f32 {
// INSERT_GENERATED_CODE_HERE
    return 1.0; // Default empty
}

// --- FDTD Simulation ---

struct SimParams {
    width: u32,
    height: u32,
    dt: f32,
    dx: f32,
    input_a_active: f32, // 1.0 or 0.0
    input_b_active: f32, // 1.0 or 0.0
    bias_active: f32,    // Always 1.0 usually
    time: f32,           // Current simulation time
};

@group(0) @binding(0) var<storage, read_write> u_current: array<f32>;
@group(0) @binding(1) var<storage, read_write> u_prev: array<f32>;
@group(0) @binding(2) var<uniform> params: SimParams;

// Index helper
fn idx(x: u32, y: u32) -> u32 {
    return y * params.width + x;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x <= 0u || x >= params.width - 1u || y <= 0u || y >= params.height - 1u) {
        return; // Boundaries
    }

    let i = idx(x, y);
    let u_c = u_current[i];
    let u_p = u_prev[i];

    // Laplacian
    let laplacian = u_current[idx(x+1u, y)] + u_current[idx(x-1u, y)] +
                    u_current[idx(x, y+1u)] + u_current[idx(x, y-1u)] - 
                    4.0 * u_c;

    // Determine Refractive Index from Geometry
    // Convert grid index (x,y) to physical coordinate 'p' for the SDF
    // Let's map grid to e.g. -10 to +10 range
    let aspect = f32(params.width) / f32(params.height);
    let uv = vec2<f32>(f32(x) / f32(params.width), f32(y) / f32(params.height));
    let p_coord = vec3<f32>((uv.x * 2.0 - 1.0) * 10.0 * aspect, (uv.y * 2.0 - 1.0) * 10.0, 0.0);
    
    let dist = map_geometry(p_coord);
    
    // Material properties
    var n = 1.0; // Air
    if (dist < 0.0) {
        n = 1.5; // Glass/High Index
    }
    
    let c = 1.0 / n;
    let courant = (c * params.dt / params.dx);
    let courant_sq = courant * courant;

    // Wave Equation
    var u_next = 2.0 * u_c - u_p + courant_sq * laplacian;
    
    // Damping (Absorbing boundaries approx)
    if (x < 10u || x > params.width - 10u || y < 10u || y > params.height - 10u) {
        u_next *= 0.9;
    } else {
        u_next *= 0.999; // Slight global loss
    }

    // --- Input Sources ---
    // Inject signals at specific locations
    let freq = 0.5; // Source frequency
    let source_val = sin(params.time * freq);
    
    // Bias Input (Top-Left)
    if (params.bias_active > 0.5 && x == 20u && y == params.height / 4u) {
        u_next = source_val; 
    }
    // Input A (Center-Left)
    if (params.input_a_active > 0.5 && x == 20u && y == params.height / 2u) {
        u_next = source_val;
    }
    // Input B (Bottom-Left)
    if (params.input_b_active > 0.5 && x == 20u && y == (params.height * 3u) / 4u) {
        u_next = source_val;
    }

    // Write to "Prev" buffer (Ping-Pong logic handled by bind group swap)
    u_prev[i] = u_next;
}
