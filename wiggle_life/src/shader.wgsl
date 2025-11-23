struct SimParams {
    width: u32,
    height: u32,
    step_type: u32, // 0 = Update Top, 1 = Update Bottom
    frame: u32,
    padding: u32,
    padding2: u32,
    padding3: u32,
    padding4: u32,
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

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    if (x >= params.width || y >= params.height) {
        return;
    }

    let idx = get_idx(x, y);
    
    // Phase 0: Update Top Layer (Orthogonal Edge Detection on Bottom)
    if (params.step_type == 0u) {
        let self_val = top_in[idx];
        
        // Read Orthogonal neighbors from BOTTOM
        let n_l = bottom_in[get_idx(x - 1u, y)];
        let n_r = bottom_in[get_idx(x + 1u, y)];
        let n_u = bottom_in[get_idx(x, y - 1u)];
        let n_d = bottom_in[get_idx(x, y + 1u)];
        
        // Edge Logic: XOR opposite pairs
        let edge_h = n_l ^ n_r;
        let edge_v = n_u ^ n_d;
        
        // If there is an edge, flip self
        let result = self_val ^ (edge_h | edge_v);
        
        top_out[idx] = result & 1u;
        bottom_out[idx] = bottom_in[idx];
    } 
    // Phase 1: Update Bottom Layer (Diagonal Edge Detection on Top)
    else {
        let self_val = bottom_in[idx];
        
        // Read Diagonal neighbors from TOP
        let n_ul = top_in[get_idx(x - 1u, y - 1u)];
        let n_ur = top_in[get_idx(x + 1u, y - 1u)];
        let n_dl = top_in[get_idx(x - 1u, y + 1u)];
        let n_dr = top_in[get_idx(x + 1u, y + 1u)];
        
        // Edge Logic: XOR opposite pairs
        let edge_d1 = n_ul ^ n_dr;
        let edge_d2 = n_ur ^ n_dl;
        
        // If there is an edge, flip self
        let result = self_val ^ (edge_d1 | edge_d2);
        
        bottom_out[idx] = result & 1u;
        top_out[idx] = top_in[idx];
    }
}