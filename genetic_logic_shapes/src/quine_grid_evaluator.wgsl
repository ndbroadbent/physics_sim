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
const ORG_DIM: u32 = 8u;   // 8x8 Organism
const SIM_STEPS: u32 = 20u;
const TEST_CASES: u32 = 64u; // Test all 64 cell positions (0..63)

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

    // 8x8 organism centered at (4,4) to (11,11) in 16x16 grid
    if (x >= 4u && x < 12u && y >= 4u && y < 12u) {
        is_organism = true;
        org_x = x - 4u;
        org_y = y - 4u;
    }

    if (is_organism) {
        let g_idx = genome_idx * 64u + (org_y * 8u + org_x);
        let g_cell = genomes[g_idx];
        grid_a[cell_idx] = SimCell(g_cell.op, g_cell.dir_a, g_cell.dir_b, 0u);
    } else {
        grid_a[cell_idx] = SimCell(16u, 0u, 0u, 0u); // 16 = VOID
    }
    
    workgroupBarrier();

    // Run Test Cases - one per cell in the 8x8 organism
    // case_id = cell index (0..63), encodes which cell's opcode we're querying
    // Input: 6 bits for cell index injected at input ports
    // Output: 4 bits for the opcode of that cell
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

                let val_a = grid_a[n_a].state;
                let val_b = grid_a[n_b].state;

                let lut_idx = (val_a << 1u) | val_b;
                res = (cell.op >> lut_idx) & 1u;
            }

            // Force Input State - row ABOVE organism (y=3, x=4..9) = 6 input bits
            // These encode the cell index (0..63) being queried
            // Input cells are NOT part of organism - they're in the void region
            if (y == 3u && x >= 4u && x < 10u) {
                let bit_idx = x - 4u;
                res = (case_id >> bit_idx) & 1u;
            }

            grid_b[cell_idx] = res;
            workgroupBarrier();

            grid_a[cell_idx].state = grid_b[cell_idx];
            workgroupBarrier();
        }

        // Read Output - row BELOW organism (y=12, x=4..7) = 4 output bits
        // These encode the opcode of the queried cell
        // Output cells are NOT part of organism - they read from the bottom edge
        if (cell_idx == 0u) {
            var out_val = 0u;
            // (4,12) -> 12*16+4 = 196
            out_val |= (grid_a[196].state << 0u);
            // (5,12) -> 197
            out_val |= (grid_a[197].state << 1u);
            // (6,12) -> 198
            out_val |= (grid_a[198].state << 2u);
            // (7,12) -> 199
            out_val |= (grid_a[199].state << 3u);

            // Store Result: 64 cases * 4 bits = 256 bits = 8 u32s per genome
            let word_offset = case_id / 8u;
            let bit_shift = (case_id % 8u) * 4u;

            atomicOr(&results[genome_idx * 8u + word_offset], out_val << bit_shift);
        }
        workgroupBarrier();
    }
}
