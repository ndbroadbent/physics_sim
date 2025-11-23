// FDTD Logic Gate Evolution Shader

// Helper functions

// --- GEOMETRY PLACEHOLDER ---
// The Rust code will inject the SdfOp::to_wgsl() function here as 'map_geometry'.
// It returns distance. d < 0 = Material (High Index), d > 0 = Air (Low Index).
fn map_geometry(p: vec3<f32>) -> f32 {
    // Apply Kaleidoscope symmetry before evaluating geometry - DISABLED FOR DEBUGGING
    // let p_sym = kaleidoscope(p);
    // return map_geometry_raw(p_sym);
    return map_geometry_raw(p);
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
        // Boundary cells still need to be updated with some value or damped.
        // For simple absorbing boundary, let's heavily damp it.
        // Don't return, let u_next be computed and then kill it.
    }

    let i = idx(x, y);
    let u_c = u_current[i];
    let u_p = u_prev[i];

    // Laplacian
    let laplacian = u_current[idx(x+1u, y)] + u_current[idx(x-1u, y)] +
                    u_current[idx(x, y+1u)] + u_current[idx(x, y-1u)] - 
                    4.0 * u_c;

    // ... (material definition) ...

    var u_next = 2.0 * u_c - u_p + courant_sq * laplacian;
    
    // Absorbing Boundary Conditions (Soft Damping Layer)
    if (x < 5u || x > params.width - 5u || y < 5u || y > params.height - 5u) {
        u_next *= 0.5; // Strong damping at edges
    }

    // --- Input Sources ---
    // Inject signals at specific locations (use constant source for DC input)
    let source_val = 1.0; // Constant ON source
    
    // Bias Input (Top Left) - located outside damping boundary now.
    if (params.bias_active > 0.5 && x == 10u && y == 16u) { u_next += source_val; }
    
    // Input A (Left Middle)
    if (params.input_a_active > 0.5 && x == 10u && y == 32u) { u_next += source_val; }
    
    // Input B (Left Bottom)
    if (params.input_b_active > 0.5 && x == 10u && y == 48u) { u_next += source_val; }

    // Write to prev (Ping-Pong logic: we bind prev as output)
    u_prev[i] = u_next;


    // Write to "Prev" buffer (Ping-Pong logic handled by bind group swap)
    u_prev[i] = u_next;
}
