struct SimParams {
    width: u32,
    height: u32,
    step_type: u32, // 0 = Update Top, 1 = Update Bottom
    padding: u32,
    padding2: u32,
    padding3: u32,
    padding4: u32,
    padding5: u32,
};

@group(0) @binding(0) var<uniform> params: SimParams;
@group(0) @binding(1) var<storage, read> top_in: array<u32>;
@group(0) @binding(2) var<storage, read_write> top_out: array<u32>;
@group(0) @binding(3) var<storage, read> bottom_in: array<u32>;
@group(0) @binding(4) var<storage, read_write> bottom_out: array<u32>;

fn get_idx(x: u32, y: u32) -> u32 {
    let w = params.width;
    let h = params.height;
    // Wrap coordinates
    let wx = (x + w) % w;
    let wy = (y + h) % h;
    return wy * w + wx;
}

// Logic Gates
fn op_nand(a: u32, b: u32) -> u32 { return 1u ^ (a & b); }
fn op_or(a: u32, b: u32) -> u32 { return a | b; }
fn op_nor(a: u32, b: u32) -> u32 { return 1u ^ (a | b); }
fn op_and(a: u32, b: u32) -> u32 { return a & b; }

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    if (x >= params.width || y >= params.height) {
        return;
    }

    let idx = get_idx(x, y);
    
    // Clockwise Neighbors: UL, U, UR, R, DR, D, DL, L
    let offsets_cw = array<vec2<i32>, 8>(
        vec2<i32>(-1, -1), // UL
        vec2<i32>( 0, -1), // U
        vec2<i32>( 1, -1), // UR
        vec2<i32>( 1,  0), // R
        vec2<i32>( 1,  1), // DR
        vec2<i32>( 0,  1), // D
        vec2<i32>(-1,  1), // DL
        vec2<i32>(-1,  0)  // L
    );

    // Counter-Clockwise Neighbors: UL, L, DL, D, DR, R, UR, U
    // (Just offsets_cw reversed, but let's be explicit for clarity)
    let offsets_ccw = array<vec2<i32>, 8>(
        vec2<i32>(-1, -1), // UL
        vec2<i32>(-1,  0), // L
        vec2<i32>(-1,  1), // DL
        vec2<i32>( 0,  1), // D
        vec2<i32>( 1,  1), // DR
        vec2<i32>( 1,  0), // R
        vec2<i32>( 1, -1), // UR
        vec2<i32>( 0, -1)  // U
    );

    // Phase 0: Update Top Layer (Favors 0, Clockwise)
    // Chain: OR -> NAND -> OR -> NAND ...
    if (params.step_type == 0u) {
        var acc = top_in[idx]; // Start with self value
        
        // Read neighbors from BOTTOM layer using CLOCKWISE offsets
        
        // Chain 0: UL (OR)
        let off0 = offsets_cw[0];
        let val0 = bottom_in[get_idx(u32(i32(x) + off0.x), u32(i32(y) + off0.y))];
        acc = op_or(acc, val0);
        
        // Chain 1: U (NAND)
        let off1 = offsets_cw[1];
        let val1 = bottom_in[get_idx(u32(i32(x) + off1.x), u32(i32(y) + off1.y))];
        acc = op_nand(acc, val1);
        
        // Chain 2: UR (OR)
        let off2 = offsets_cw[2];
        let val2 = bottom_in[get_idx(u32(i32(x) + off2.x), u32(i32(y) + off2.y))];
        acc = op_or(acc, val2);
        
        // Chain 3: R (NAND)
        let off3 = offsets_cw[3];
        let val3 = bottom_in[get_idx(u32(i32(x) + off3.x), u32(i32(y) + off3.y))];
        acc = op_nand(acc, val3);
        
        // Chain 4: DR (OR)
        let off4 = offsets_cw[4];
        let val4 = bottom_in[get_idx(u32(i32(x) + off4.x), u32(i32(y) + off4.y))];
        acc = op_or(acc, val4);
        
        // Chain 5: D (NAND)
        let off5 = offsets_cw[5];
        let val5 = bottom_in[get_idx(u32(i32(x) + off5.x), u32(i32(y) + off5.y))];
        acc = op_nand(acc, val5);
        
        // Chain 6: DL (OR)
        let off6 = offsets_cw[6];
        let val6 = bottom_in[get_idx(u32(i32(x) + off6.x), u32(i32(y) + off6.y))];
        acc = op_or(acc, val6);
        
        // Chain 7: L (NAND) -> Final Result
        let off7 = offsets_cw[7];
        let val7 = bottom_in[get_idx(u32(i32(x) + off7.x), u32(i32(y) + off7.y))];
        acc = op_nand(acc, val7);
        
        top_out[idx] = acc & 1u;
        bottom_out[idx] = bottom_in[idx];
    } 
    // Phase 1: Update Bottom Layer (Favors 1, Counter-Clockwise)
    // Chain: AND -> NOR -> AND -> NOR ...
    else {
        var acc = bottom_in[idx]; // Start with self value
        
        // Read neighbors from TOP layer using COUNTER-CLOCKWISE offsets
        
        // Chain 0: UL (AND)
        let off0 = offsets_ccw[0];
        let val0 = top_in[get_idx(u32(i32(x) + off0.x), u32(i32(y) + off0.y))];
        acc = op_and(acc, val0);
        
        // Chain 1: L (NOR)
        let off1 = offsets_ccw[1];
        let val1 = top_in[get_idx(u32(i32(x) + off1.x), u32(i32(y) + off1.y))];
        acc = op_nor(acc, val1);
        
        // Chain 2: DL (AND)
        let off2 = offsets_ccw[2];
        let val2 = top_in[get_idx(u32(i32(x) + off2.x), u32(i32(y) + off2.y))];
        acc = op_and(acc, val2);
        
        // Chain 3: D (NOR)
        let off3 = offsets_ccw[3];
        let val3 = top_in[get_idx(u32(i32(x) + off3.x), u32(i32(y) + off3.y))];
        acc = op_nor(acc, val3);
        
        // Chain 4: DR (AND)
        let off4 = offsets_ccw[4];
        let val4 = top_in[get_idx(u32(i32(x) + off4.x), u32(i32(y) + off4.y))];
        acc = op_and(acc, val4);
        
        // Chain 5: R (NOR)
        let off5 = offsets_ccw[5];
        let val5 = top_in[get_idx(u32(i32(x) + off5.x), u32(i32(y) + off5.y))];
        acc = op_nor(acc, val5);
        
        // Chain 6: UR (AND)
        let off6 = offsets_ccw[6];
        let val6 = top_in[get_idx(u32(i32(x) + off6.x), u32(i32(y) + off6.y))];
        acc = op_and(acc, val6);
        
        // Chain 7: U (NOR) -> Final Result
        let off7 = offsets_ccw[7];
        let val7 = top_in[get_idx(u32(i32(x) + off7.x), u32(i32(y) + off7.y))];
        acc = op_nor(acc, val7);
        
        bottom_out[idx] = acc & 1u;
        top_out[idx] = top_in[idx];
    }
}
