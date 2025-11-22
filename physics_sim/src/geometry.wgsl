// 3D Signed Distance Function (SDF) Library for Genetic Structures

// A Genome defines the parameters for constructing the geometry
struct Genome {
    params: array<f32, 16>, // 16 genes to tweak geometry
};

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

// --- Fractal Generation ---

// A simple Menger Sponge-like iteration or similar IFS
fn sdFractalChannel(pos: vec3<f32>, genes: Genome) -> f32 {
    var p = pos;
    var d = sdBox(p, vec3<f32>(100.0)); // Bounding box

    // Iterative folding (Fractal)
    let iterations = 4;
    let scale = genes.params[0] * 2.0 + 1.0; // Gene 0: Scale
    let offset = vec3<f32>(genes.params[1], genes.params[2], genes.params[3]) * 5.0; // Genes 1-3: Offset
    let rot_angle = genes.params[4] * 3.14159; // Gene 4: Rotation

    for (var i = 0; i < iterations; i++) {
        p = abs(p + offset) - offset; // Fold space
        p = rotY(p, rot_angle);       // Rotate
        p = p * scale;                // Scale
        d = min(d, sdBox(p, vec3<f32>(1.0)) / pow(scale, f32(i + 1)));
    }
    
    // Create a "Channel" by subtracting a cylinder or path through it?
    // Or the structure *is* the channel walls.
    // Let's say positive distance = solid, negative = void (channel).
    
    // Invert so we are inside the sponge?
    return -d;
}

// Main evaluation function
// Returns distance to nearest surface.
// Input: p = position in 3D space (local to the test chamber)
//        genes = the DNA defining the shape
fn map_geometry(p: vec3<f32>, genes: Genome) -> f32 {
    // Gene 5: Select primitive or fractal type? For now, hardcode fractal.
    return sdFractalChannel(p, genes);
}
