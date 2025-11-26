struct Node {
    op: u32,
    in_a: u32,
    in_b: u32,
    _pad: u32,
}

struct GenomeInfo {
    output_node_idx: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

@group(0) @binding(0) var<storage, read> genomes: array<Node>; 
@group(0) @binding(1) var<storage, read> genome_infos: array<GenomeInfo>;
@group(0) @binding(2) var<storage, read> target_data: array<u32>;
@group(0) @binding(3) var<storage, read> inputs: array<u32>; // [Chunk0_In0..In16, Chunk1...]
@group(0) @binding(4) var<storage, read_write> errors: array<atomic<u32>>;

const NUM_NODES: u32 = 600u;
const NUM_INPUTS: u32 = 17u; // 8 X, 8 Y, 1 ShapeID
const CHUNKS_PER_IMAGE: u32 = 2048u; 

// Helper for bit counting
fn count_set_bits(n: u32) -> u32 {
    return countOneBits(n);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let total_chunks = arrayLength(&target_data);
    let num_genomes = arrayLength(&genome_infos);
    
    let genome_idx = global_id.y;
    let chunk_idx = global_id.x;

    if (genome_idx >= num_genomes || chunk_idx >= CHUNKS_PER_IMAGE) {
        return;
    }

    // Load Inputs
    // Inputs are stored flat: Chunk0[17], Chunk1[17]...
    // We need to copy them to our local values array
    var values: array<u32, 617>; // 17 inputs + 600 nodes
    let input_offset = chunk_idx * NUM_INPUTS;
    
    for (var i = 0u; i < NUM_INPUTS; i++) {
        values[i] = inputs[input_offset + i];
    }

    // Execute Genome
    let genome_offset = genome_idx * NUM_NODES;
    
    for (var i = 0u; i < NUM_NODES; i++) {
        let node = genomes[genome_offset + i];
        let val_a = values[node.in_a]; // inputs are 0..16, nodes start at 17. Wait.
        // In CPU code: "Inputs to a node can be any previous node OR system input"
        // CPU: buffer[0..16] are inputs. buffer[17] is node 0.
        // GPU: values[0..16] are inputs. values[17] is node 0.
        // So node.in_a/in_b indices are already correct (0-based into the values array)
        
        let b_val_b = values[node.in_b]; // avoid naming collision

        var res = 0u;
        switch node.op {
            case 0u: { res = val_a & b_val_b; }       // AND
            case 1u: { res = val_a | b_val_b; }       // OR
            case 2u: { res = val_a ^ b_val_b; }       // XOR
            case 3u: { res = ~(val_a & b_val_b); }    // NAND
            case 4u: { res = ~(val_a | b_val_b); }    // NOR
            case 5u: { res = ~val_a; }                // NOT (uses A)
            case 6u: { res = val_a; }                 // WIRE
            default: { res = 0u; }
        }
        values[NUM_INPUTS + i] = res;
    }

    // Get Output
    let info = genome_infos[genome_idx];
    let output = values[info.output_node_idx];
    
    // Compare with Target
    let expected = target_data[chunk_idx];
    let diff = output ^ expected;
    let error_count = count_set_bits(diff);
    
    // Atomic Add to Error Buffer
    atomicAdd(&errors[genome_idx], error_count);
}
