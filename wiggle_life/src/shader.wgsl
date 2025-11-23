struct SimParams {
    width: u32,
    height: u32,
    offset_x: i32,
    offset_y: i32,
    step_type: u32, // 0 = Update Top, 1 = Update Bottom
    frame: u32, // Added frame parameter
    pad1: u32,
    pad2: u32,
};

@group(0) @binding(0) var<uniform> params: SimParams;
@group(0) @binding(1) var<storage, read> top_in: array<u32>;
@group(0) @binding(2) var<storage, read_write> top_out: array<u32>;
@group(0) @binding(3) var<storage, read> bottom_in: array<u32>;
@group(0) @binding(4) var<storage, read_write> bottom_out: array<u32>;

fn get_idx(x: i32, y: i32) -> u32 {
    let w = i32(params.width);
    let h = i32(params.height);
    // Wrap coordinates
    let wx = ((x % w) + w) % w;
    let wy = ((y % h) + h) % h;
    return u32(wy * w + wx);
}

// Simple Hash Function for Deterministic "Randomness"
fn hash(x: u32, y: u32, t: u32) -> f32 {
    var state = x * 747796405u + y * 2891336453u + t * 13u;
    var word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    word = (word >> 22u) ^ word;
    return f32(word) / 4294967295.0;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);

    if (x >= i32(params.width) || y >= i32(params.height)) {
        return;
    }

    let idx = get_idx(x, y);

    let is_inflation = params.frame < 2000u;
    let quantum_alpha = 0.95; // chance of tunneling
    let rnd = hash(u32(x), u32(y), params.frame);
    let is_tunneling = rnd < quantum_alpha;

    let use_xor_logic = is_inflation || is_tunneling;

    // Logic Split based on which layer we are updating
    if (params.step_type == 0u || params.step_type == 2u || params.step_type == 5u) {
        // --- Update Top Layer ---

        let val_top = top_in[idx];

        let shifted_x = x + params.offset_x;
        let shifted_y = y + params.offset_y;
        let val_bottom = bottom_in[get_idx(shifted_x, shifted_y)];

        var result = 0u;
        if (params.step_type == 0u) { // AND
            result = u32(val_top != 0u && val_bottom != 0u);
        } else if (params.step_type == 2u) { // NOR
            result = u32(!(val_top != 0u || val_bottom != 0u));
        } else { // XNOR (5) or NOR (Standard Model)
            if (use_xor_logic) {
                result = u32((val_top != 0u) == (val_bottom != 0u)); // XNOR
            } else {
                result = u32(!(val_top != 0u || val_bottom != 0u)); // NOR
            }
        }

        top_out[idx] = result;
        bottom_out[idx] = bottom_in[idx]; // Pass-through

    } else {
        // --- Update Bottom Layer ---

        let val_bottom = bottom_in[idx];

        let shifted_x = x - params.offset_x;
        let shifted_y = y - params.offset_y;
        let val_top = top_in[get_idx(shifted_x, shifted_y)];

        var result = 0u;
        if (params.step_type == 1u) { // OR
            result = u32(val_top != 0u || val_bottom != 0u);
        } else if (params.step_type == 3u) { // NAND
            result = u32(!(val_top != 0u && val_bottom != 0u));
        } else { // XOR (4) or NAND (Standard Model)
            if (use_xor_logic) {
                result = u32((val_top != 0u) != (val_bottom != 0u)); // XOR
            } else {
                result = u32(!(val_top != 0u && val_bottom != 0u)); // NAND
            }
        }

        bottom_out[idx] = result;
        top_out[idx] = top_in[idx]; // Pass-through
    }
}
