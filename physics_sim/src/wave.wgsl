// FDTD Wave Solver Shader

// We use a 2D grid.
// u_current: Amplitude at time t
// u_prev: Amplitude at time t-1
// material: x = Refractive Index Base (n0), y = Nonlinear Coeff (n2), z = Loss/Absorption

struct WaveParams {
    width: u32,
    height: u32,
    dt: f32,
    dx: f32,
    damp: f32, // Global damping
};

@group(0) @binding(0) var<storage, read_write> u_current: array<f32>;
@group(0) @binding(1) var<storage, read_write> u_prev: array<f32>;
@group(0) @binding(2) var<storage, read_write> material: array<vec4<f32>>; // Use vec4 for alignment
@group(0) @binding(3) var<uniform> params: WaveParams;

// Index helper
fn idx(x: u32, y: u32) -> u32 {
    return y * params.width + x;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x <= 0u || x >= params.width - 1u || y <= 0u || y >= params.height - 1u) {
        // Boundary conditions (Simple absorbing/zero for now)
        return;
    }

    let i = idx(x, y);
    
    let u_c = u_current[i];
    let u_p = u_prev[i];
    let mat = material[i]; // x=n0, y=n2 (Kerr)

    // Laplacian (Finite Difference)
    // d2u/dx2 + d2u/dy2
    let laplacian = u_current[idx(x+1u, y)] + u_current[idx(x-1u, y)] +
                    u_current[idx(x, y+1u)] + u_current[idx(x, y-1u)] - 
                    4.0 * u_c;

    // Wave Speed Calculation (The Magic)
    // c = 1 / n
    // n = n0 + n2 * Intensity
    let intensity = u_c * u_c;
    let n = mat.x + mat.y * intensity;
    let c = 1.0 / max(n, 1.0); // Vacuum c=1.0. n>=1.0

    // Wave Equation Update: u_next = 2*u_curr - u_prev + (c*dt/dx)^2 * laplacian
    // CFL condition: c*dt/dx < 0.707 for stability
    let courant = (c * params.dt / params.dx);
    let courant_sq = courant * courant;

    var u_next = 2.0 * u_c - u_p + courant_sq * laplacian;
    
    // Damping (Loss)
    u_next *= params.damp;

    // Write to u_prev (which acts as u_next for the next frame swap, 
    // but here we need a third buffer or swap logic. 
    // Standard FDTD ping-pongs.
    // Here, we assume the host swaps binding 0 and 1 every frame.
    // So we write the result to 'u_prev' which will be 'u_current' next frame?
    // No, we write to a separate output or overwrite u_prev if we are careful.
    
    // Wait, for standard ping-pong:
    // Read Current, Read Prev -> Write Next.
    // We need 3 buffers or 2 buffers with specific logic.
    // Let's assume we write to u_prev, effectively destroying history t-1.
    // But we need t-1 for the calculation. We read it into 'u_p'. 
    // So we can overwrite u_prev array with u_next value.
    // Next frame: Bind 1 becomes "Current", Bind 0 becomes "Next" (reusing old current).
    // So we write to PREV buffer? No, that's confusing.
    
    // Let's assume:
    // Binding 0: Current State (t)
    // Binding 1: Previous State (t-1)  <-- BECOMES Next State (t+1)
    
    u_prev[i] = u_next;
}
