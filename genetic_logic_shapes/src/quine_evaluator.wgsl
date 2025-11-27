struct Node {
    table: u32, 
    in_a: u32,
    in_b: u32,
    _pad: u32,
}

struct GenomeInfo {
    output_node_idx: u32,
    target_start_bit: u32,
    target_len_bits: u32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> genomes: array<Node>; 
@group(0) @binding(1) var<storage, read> genome_infos: array<GenomeInfo>;
@group(0) @binding(2) var<storage, read> targets: array<u32>; 
@group(0) @binding(3) var<storage, read_write> errors: array<atomic<u32>>;

const NUM_NODES: u32 = 100u;
const INPUT_BITS: u32 = 11u; 

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let genome_idx = global_id.x;
    let num_genomes = arrayLength(&genome_infos);
    
    if (genome_idx >= num_genomes) {
        return;
    }

    let genome_offset = genome_idx * NUM_NODES;
    let info = genome_infos[genome_idx];
    
    var total_error = 0u;
    var values: array<u32, 111>; // 11 inputs + 100 nodes

    // Iterate through the target bits (Self-Description)
    for (var i = 0u; i < info.target_len_bits; i++) {
        // 1. Get Target Bit
        let global_bit_idx = info.target_start_bit + i;
        let word_idx = global_bit_idx / 32u;
        let bit_offset = global_bit_idx % 32u;
        let expected_bit = (targets[word_idx] >> bit_offset) & 1u;

        // 2. Set Inputs (Counter 'i')
        for (var b = 0u; b < INPUT_BITS; b++) {
            values[b] = (i >> b) & 1u;
        }

        // 3. Run Circuit
        for (var n = 0u; n < NUM_NODES; n++) {
            let node = genomes[genome_offset + n];
            let val_a = values[node.in_a];
            let val_b = values[node.in_b];
            
            let lut_idx = (val_a << 1u) | val_b;
            let res = (node.table >> lut_idx) & 1u;
            
            values[INPUT_BITS + n] = res;
        }

        // 4. Check Output
        let output = values[info.output_node_idx];
        if (output != expected_bit) {
            total_error += 1u;
        }
    }
    
    atomicStore(&errors[genome_idx], total_error);
}