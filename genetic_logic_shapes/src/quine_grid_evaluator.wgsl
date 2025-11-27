struct Cell {
    op: u32,      
    dir_a: u32,   
    dir_b: u32,   
    _pad: u32,
}

// No IOConfig needed, hardcoded relative ports

@group(0) @binding(0) var<storage, read> genomes: array<Cell>; // [PopSize * 16]
// Binding 1 removed (IOConfig)
@group(0) @binding(1) var<storage, read_write> results: array<atomic<u32>>; // [PopSize * 16] (1 result word per case? No, 16 cases * 4 bits = 64 bits = 2 u32s)
// Let's just output errors directly to save bandwidth?
// "Fitness function will not be one fixed value... we want to see what that structure looks like."
// We need the actual outputs to compare on CPU or compute error on GPU.
// Let's compute ERROR on GPU to be fast.
// We pass the TARGET (the genome itself) to the shader?
// Or we just read the genome from `genomes` buffer! Ideally.
// Let's write outputs to buffer.
// 16 cases. Each produces 4 bits. Total 64 bits = 2 u32s per genome.

const GRID_DIM: u32 = 16u; // 16x16 Simulation Grid
const ORG_DIM: u32 = 4u;   // 4x4 Organism
const SIM_STEPS: u32 = 20u; 
const TEST_CASES: u32 = 16u; // 0..15 positions

struct SimCell {
    op: u32,
    dir_a: u32,
    dir_b: u32,
    state: u32,
}

var<workgroup> grid_a: array<SimCell, 256>;
var<workgroup> grid_b: array<u32, 256>; // Only need state for B

fn get_neighbor_idx(idx: u32, dir: u32) -> u32 {
    let x = i32(idx % GRID_DIM);
    let y = i32(idx / GRID_DIM);
    var dx = 0i; var dy = 0i;
    if (dir == 0u || dir == 1u || dir == 7u) { dy = -1i; }
    if (dir == 3u || dir == 4u || dir == 5u) { dy = 1i; }
    if (dir == 1u || dir == 2u || dir == 3u) { dx = 1i; }
    if (dir == 5u || dir == 6u || dir == 7u) { dx = -1i; }
    
    let nx = (x + dx + i32(GRID_DIM)) % i32(GRID_DIM);
    let ny = (y + dy + i32(GRID_DIM)) % i32(GRID_DIM);
    return u32(ny) * GRID_DIM + u32(nx);
}

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) group_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let genome_idx = group_id.x;
    let cell_idx = local_id.x; // 0..255
    
    // 1. Load Genome into Workgroup Memory (Centered)
    // Offset (6,6). Indices: row 6..9, col 6..9.
    // row * 16 + col.
    // 6*16+6 = 102.
    
    let x = cell_idx % GRID_DIM;
    let y = cell_idx / GRID_DIM;
    
    var is_organism = false;
    var org_x = 0u;
    var org_y = 0u;
    
    if (x >= 6u && x < 10u && y >= 6u && y < 10u) {
        is_organism = true;
        org_x = x - 6u;
        org_y = y - 6u;
    }
    
    if (is_organism) {
        let g_idx = genome_idx * 16u + (org_y * 4u + org_x);
        let g_cell = genomes[g_idx];
        grid_a[cell_idx] = SimCell(g_cell.op, g_cell.dir_a, g_cell.dir_b, 0u);
    } else {
        grid_a[cell_idx] = SimCell(16u, 0u, 0u, 0u); // 16 = VOID
    }
    
    workgroupBarrier();

    // Run Test Cases
    for (var case_id = 0u; case_id < TEST_CASES; case_id++) {
        // Reset State
        grid_a[cell_idx].state = 0u;
        workgroupBarrier();
        
        // Run Simulation
        for (var step = 0u; step < SIM_STEPS; step++) {
            let cell = grid_a[cell_idx];
            var res = 0u;
            
            if (cell.op < 16u) {
                let n_a = get_neighbor_idx(cell_idx, cell.dir_a);
                let n_b = get_neighbor_idx(cell_idx, cell.dir_b);
                
                var val_a = grid_a[n_a].state;
                var val_b = grid_a[n_b].state;
                
                // Inject Inputs (Override neighbors if they are input ports?)
                // Input Port: Top Edge of Organism (Relative (0,0)..(3,0))
                // Absolute: (6,6)..(9,6)
                // Let's say Input 0 is injected at (6,6), Input 1 at (7,6)...
                // Wait, we need 4 bits of input (Index 0..15).
                // (6,6) gets Bit 0. (7,6) gets Bit 1. (8,6) gets Bit 2. (9,6) gets Bit 3.
                
                if (y == 6u && x >= 6u && x < 10u) {
                    let bit_idx = x - 6u;
                    // For input ports, we assume they READ from the "outside world".
                    // So if a cell connects to them, it reads the input.
                    // But here `val_a` reads from neighbor.
                    // If `n_a` points to an input cell, does it get the input?
                    // Let's say Input Cells FORCE their state.
                }
                
                let lut_idx = (val_a << 1u) | val_b;
                res = (cell.op >> lut_idx) & 1u;
            }
            
            // Force Input State
            if (y == 6u && x >= 6u && x < 10u) {
                let bit_idx = x - 6u;
                res = (case_id >> bit_idx) & 1u;
            }
            
            grid_b[cell_idx] = res;
            workgroupBarrier();
            
            grid_a[cell_idx].state = grid_b[cell_idx];
            workgroupBarrier();
        }
        
        // Read Output
        // Output Port: Bottom Edge of Organism (Relative (0,3)..(3,3)) -> (6,9)..(9,9)
        // We read 4 bits to form the Opcode (0..15).
        // Wait, we need to output 4 bits.
        // Bit 0 at (6,9), Bit 1 at (7,9)...
        
        if (cell_idx == 0u) {
            var out_val = 0u;
            // Read (6,9) -> 9*16+6 = 150
            out_val |= (grid_a[150].state << 0u);
            // Read (7,9) -> 151
            out_val |= (grid_a[151].state << 1u);
            // Read (8,9) -> 152
            out_val |= (grid_a[152].state << 2u);
            // Read (9,9) -> 153
            out_val |= (grid_a[153].state << 3u);
            
            // Store Result
            // 2 u32s per genome. 
            // case_id 0..15.
            // Each u32 holds 8 cases (4 bits * 8 = 32 bits).
            let word_offset = case_id / 8u;
            let bit_shift = (case_id % 8u) * 4u;
            
            atomicOr(&results[genome_idx * 2u + word_offset], out_val << bit_shift);
        }
        workgroupBarrier();
    }
}
