// 16x16 Quine Evaluator
//
// Tests if an organism can describe its own structure.
// For each of 256 cells, we inject the cell index as input and expect
// the organism to output that cell's 12-bit descriptor.

struct Cell {
    op: u32,
    dir_a: u32,
    dir_b: u32,
    flags: u32,  // bit 0: is_input, bit 1: is_output
}

@group(0) @binding(0) var<storage, read> genomes: array<Cell>;
@group(0) @binding(1) var<storage, read_write> results: array<atomic<u32>>;

const GRID_DIM: u32 = 16u;
const GENOME_SIZE: u32 = 256u;
const SIM_STEPS: u32 = 30u;
const TEST_CASES: u32 = 256u;
const RESULTS_U32S: u32 = 96u;  // 256 * 12 bits = 3072 bits = 96 u32s

// Operation constants
const OP_VOID: u32 = 16u;
const OP_JUMP: u32 = 17u;

// Jump direction constants (4-way cardinal)
const JUMP_N: u32 = 0u;
const JUMP_E: u32 = 1u;
const JUMP_S: u32 = 2u;
const JUMP_W: u32 = 3u;

struct SimCell {
    op: u32,
    dir_a: u32,
    dir_b: u32,
    flags: u32,
    state: u32,
}

var<workgroup> grid_a: array<SimCell, 256>;
var<workgroup> grid_b: array<u32, 256>;

// Count input cells and get their indices (up to 16)
var<workgroup> input_count: atomic<u32>;
var<workgroup> input_indices: array<u32, 16>;
var<workgroup> output_count: atomic<u32>;
var<workgroup> output_indices: array<u32, 16>;

fn get_neighbor_idx(idx: u32, dir: u32) -> u32 {
    let x = i32(idx % GRID_DIM);
    let y = i32(idx / GRID_DIM);
    var dx = 0i;
    var dy = 0i;

    // Direction encoding: 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW
    if (dir == 0u || dir == 1u || dir == 7u) { dy = -1i; }
    if (dir == 3u || dir == 4u || dir == 5u) { dy = 1i; }
    if (dir == 1u || dir == 2u || dir == 3u) { dx = 1i; }
    if (dir == 5u || dir == 6u || dir == 7u) { dx = -1i; }

    let nx = (x + dx + i32(GRID_DIM)) % i32(GRID_DIM);
    let ny = (y + dy + i32(GRID_DIM)) % i32(GRID_DIM);
    return u32(ny) * GRID_DIM + u32(nx);
}

/// Get jump target index for OP_JUMP cells
/// dir_a = direction (0=N, 1=E, 2=S, 3=W)
/// dir_b = distance (2, 3, or 4)
fn get_jump_target_idx(idx: u32, direction: u32, distance: u32) -> u32 {
    let x = i32(idx % GRID_DIM);
    let y = i32(idx / GRID_DIM);
    let dist = i32(distance);
    var dx = 0i;
    var dy = 0i;

    if (direction == JUMP_N) { dy = -dist; }
    else if (direction == JUMP_E) { dx = dist; }
    else if (direction == JUMP_S) { dy = dist; }
    else if (direction == JUMP_W) { dx = -dist; }

    let nx = (x + dx + i32(GRID_DIM)) % i32(GRID_DIM);
    let ny = (y + dy + i32(GRID_DIM)) % i32(GRID_DIM);
    return u32(ny) * GRID_DIM + u32(nx);
}

/// Encode a cell as 12 bits: [op:4][dir_a:3][dir_b:3][is_input:1][is_output:1]
fn encode_cell(cell_idx: u32, genome_offset: u32) -> u32 {
    let cell = genomes[genome_offset + cell_idx];
    let op = min(cell.op, 15u);  // Clamp to 4 bits
    let dir_a = cell.dir_a & 7u;
    let dir_b = cell.dir_b & 7u;
    let is_in = (cell.flags & 1u);
    let is_out = (cell.flags >> 1u) & 1u;
    return (op << 8u) | (dir_a << 5u) | (dir_b << 2u) | (is_in << 1u) | is_out;
}

@compute @workgroup_size(256)
fn main(
    @builtin(workgroup_id) group_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let genome_idx = group_id.x;
    let cell_idx = local_id.x;
    let genome_offset = genome_idx * GENOME_SIZE;

    // Load genome into workgroup memory
    let g_cell = genomes[genome_offset + cell_idx];
    grid_a[cell_idx] = SimCell(g_cell.op, g_cell.dir_a, g_cell.dir_b, g_cell.flags, 0u);

    // Initialize I/O counters
    if (cell_idx == 0u) {
        atomicStore(&input_count, 0u);
        atomicStore(&output_count, 0u);
    }
    workgroupBarrier();

    // Collect input and output cell indices
    if ((g_cell.flags & 1u) != 0u) {
        let idx = atomicAdd(&input_count, 1u);
        if (idx < 16u) {
            input_indices[idx] = cell_idx;
        }
    }
    if ((g_cell.flags & 2u) != 0u) {
        let idx = atomicAdd(&output_count, 1u);
        if (idx < 16u) {
            output_indices[idx] = cell_idx;
        }
    }
    workgroupBarrier();

    let num_inputs = min(atomicLoad(&input_count), 16u);
    let num_outputs = min(atomicLoad(&output_count), 16u);

    // Run test cases: for each cell, query its descriptor
    for (var test_cell = 0u; test_cell < TEST_CASES; test_cell++) {
        // Reset state
        grid_a[cell_idx].state = 0u;
        workgroupBarrier();

        // Run simulation
        for (var step = 0u; step < SIM_STEPS; step++) {
            let cell = grid_a[cell_idx];
            var res = 0u;

            if (cell.op < 16u) {
                // Logic gate: read two neighbors, apply 4-bit LUT
                let n_a = get_neighbor_idx(cell_idx, cell.dir_a);
                let n_b = get_neighbor_idx(cell_idx, cell.dir_b);

                let val_a = grid_a[n_a].state;
                let val_b = grid_a[n_b].state;

                // 4-bit LUT: op encodes truth table
                let lut_idx = (val_a << 1u) | val_b;
                res = (cell.op >> lut_idx) & 1u;
            } else if (cell.op == OP_JUMP) {
                // Jump wire: read from distant cell
                // dir_a = direction (N/E/S/W), dir_b = distance (2,3,4)
                let jump_dest = get_jump_target_idx(cell_idx, cell.dir_a, cell.dir_b);
                res = grid_a[jump_dest].state;
            }
            // OP_VOID (16) and unknown ops output 0 (res already 0)

            // Input cells receive the test_cell index bits
            // Distribute 8 bits of test_cell across input cells
            if ((cell.flags & 1u) != 0u) {
                // Find which input cell this is (0..num_inputs-1)
                for (var i = 0u; i < num_inputs; i++) {
                    if (input_indices[i] == cell_idx) {
                        // This input cell gets bit i of test_cell
                        res = (test_cell >> i) & 1u;
                        break;
                    }
                }
            }

            grid_b[cell_idx] = res;
            workgroupBarrier();

            grid_a[cell_idx].state = grid_b[cell_idx];
            workgroupBarrier();
        }

        // Read output - collect bits from output cells
        if (cell_idx == 0u) {
            var output_val = 0u;

            // Collect up to 12 bits from output cells
            for (var i = 0u; i < min(num_outputs, 12u); i++) {
                let out_cell_idx = output_indices[i];
                let bit = grid_a[out_cell_idx].state & 1u;
                output_val |= (bit << i);
            }

            // Store the 12-bit result
            // Results are packed: each test_cell produces 12 bits
            let bit_offset = test_cell * 12u;
            let word_idx = bit_offset / 32u;
            let bit_in_word = bit_offset % 32u;

            let result_base = genome_idx * RESULTS_U32S;

            if (bit_in_word + 12u <= 32u) {
                // Fits in one word
                atomicOr(&results[result_base + word_idx], output_val << bit_in_word);
            } else {
                // Spans two words
                let lo_bits = 32u - bit_in_word;
                atomicOr(&results[result_base + word_idx], output_val << bit_in_word);
                atomicOr(&results[result_base + word_idx + 1u], output_val >> lo_bits);
            }
        }
        workgroupBarrier();
    }
}
