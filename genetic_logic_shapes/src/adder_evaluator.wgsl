struct Node {
    op: u32,
    in_a: u32,
    in_b: u32,
    _pad: u32,
}

// 3 Outputs + Padding
struct GenomeInfo {
    out_0: u32,
    out_1: u32,
    out_2: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
    _pad4: u32,
    _pad5: u32,
}

@group(0) @binding(0) var<storage, read> genomes: array<Node>; 
@group(0) @binding(1) var<storage, read> genome_infos: array<GenomeInfo>;
@group(0) @binding(2) var<storage, read_write> errors: array<atomic<u32>>;

const NUM_NODES: u32 = 100u; // Smaller network needed
const NUM_INPUTS: u32 = 4u; 

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
    var values: array<u32, 104>; // 4 inputs + 100 nodes

    // Iterate all 16 cases (4 bits total input)
    for (var a = 0u; a < 4u; a++) {
        for (var b = 0u; b < 4u; b++) {
            // Set Inputs (2 bits each)
            values[0] = (a >> 0u) & 1u;
            values[1] = (a >> 1u) & 1u;
            
            values[2] = (b >> 0u) & 1u;
            values[3] = (b >> 1u) & 1u;

            // Execute Circuit
            for (var i = 0u; i < NUM_NODES; i++) {
                let node = genomes[genome_offset + i];
                let val_a = values[node.in_a];
                let val_b = values[node.in_b];

                var res = 0u;
                switch node.op {
                    case 0u: { res = val_a & val_b; }       // AND
                    case 1u: { res = val_a | val_b; }       // OR
                    case 2u: { res = val_a ^ val_b; }       // XOR
                    case 3u: { res = ~(val_a & val_b); }    // NAND
                    case 4u: { res = ~(val_a | val_b); }    // NOR
                    case 5u: { res = ~val_a; }              // NOT
                    case 6u: { res = val_a; }               // WIRE
                    default: { res = 0u; }
                }
                values[NUM_INPUTS + i] = res & 1u;
            }

            // Reconstruct Output (3 bits)
            var out_val = 0u;
            out_val |= (values[info.out_0] << 0u);
            out_val |= (values[info.out_1] << 1u);
            out_val |= (values[info.out_2] << 2u);

            let expected = a + b;
            let diff = out_val ^ expected;
            
            // Weighted Error
            if ((diff & 1u) != 0u) { total_error += 4u; } // Bit 0
            if ((diff & 2u) != 0u) { total_error += 2u; } // Bit 1
            if ((diff & 4u) != 0u) { total_error += 1u; } // Bit 2 (Carry)
        }
    }
    
    atomicStore(&errors[genome_idx], total_error);
}
