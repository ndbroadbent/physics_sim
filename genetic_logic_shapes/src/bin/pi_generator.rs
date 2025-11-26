#[path = "../gate_multi.rs"]
mod gate_multi;

use gate_multi::Genome;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand::Rng;

const PI_DIGITS: [u8; 7] = [3, 1, 4, 1, 5, 9, 2]; // 3.141592
const NUM_DP_INPUT_BITS: usize = 3; // For 7 indices (0-6), we need 3 bits
const NUM_OUTPUT_BITS: usize = 8; // To represent a digit 0-9

const NUM_NODES: usize = 300; // Increased nodes for more complex function
const POPULATION_SIZE: usize = 1000; // Increased population

fn main() {
    let mut rng = ChaCha8Rng::seed_from_u64(12345); // Fixed seed for reproducibility
    
    let mut parent = Genome::new_random(NUM_DP_INPUT_BITS, NUM_NODES, NUM_OUTPUT_BITS, &mut rng);
    let mut gen = 0;
    
    println!("Target Pi Digits (Binary):");
    for &digit in &PI_DIGITS {
        println!("  {} -> {:08b}", digit, digit);
    }

    let mut stagnant_gens = 0;

    loop {
        let mut best_child = parent.clone();
        let mut best_fitness = evaluate(&best_child);
        
        // Adaptive Mutation
        let mutation_rate = if stagnant_gens < 20 { 0.05 } else if stagnant_gens < 50 { 0.20 } else { 0.50 };

        // 1+N Evolution Strategy
        for _ in 0..POPULATION_SIZE {
            let mut child = parent.clone();
            child.mutate(mutation_rate, &mut rng); 
            let fitness = evaluate(&child);
            
            if fitness < best_fitness {
                best_child = child;
                best_fitness = fitness;
            }
        }
        
        if best_fitness < evaluate(&parent) {
            parent = best_child;
            stagnant_gens = 0;
        } else {
            stagnant_gens += 1;
            // Kick: If stuck for too long, Hard Reset
            if stagnant_gens > 60 {
                println!("Kick! Hard Reset. Stuck at fitness {}", evaluate(&parent));
                parent = Genome::new_random(NUM_DP_INPUT_BITS, NUM_NODES, NUM_OUTPUT_BITS, &mut rng);
                stagnant_gens = 0;
            }
        }
        
        if gen % 100 == 0 {
            println!("Gen {:04} | Error: {} | Rate: {:.2} | Outputs: {:?}", gen, best_fitness, mutation_rate, get_all_outputs(&parent));
        }
        
        if best_fitness == 0 {
            println!("SOLVED at Gen {}!", gen);
            println!("Circuit Outputs:");
            for i in 0..PI_DIGITS.len() {
                let dp_input_bools = u8_to_bool_array(i as u8, NUM_DP_INPUT_BITS);
                let outputs = parent.eval(&dp_input_bools);
                let val = bools_to_u8(&outputs);
                println!("  Input {:?} ({}) -> Output {} ({:08b}) (Target {} ({:08b}))", 
                    dp_input_bools, i, val, val, PI_DIGITS[i], PI_DIGITS[i]);
            }
            save_dot(&parent, "pi_generator.dot");
            break;
        }
        
        gen += 1;
    }
}

// Evaluates a genome against all PI_DIGITS and returns total Hamming error
fn evaluate(genome: &Genome) -> u32 {
    let mut total_hamming_error = 0;
    for i in 0..PI_DIGITS.len() {
        let dp_input_bools = u8_to_bool_array(i as u8, NUM_DP_INPUT_BITS);
        let outputs = genome.eval(&dp_input_bools);
        let val = bools_to_u8(&outputs);
        total_hamming_error += (val ^ PI_DIGITS[i]).count_ones();
    }
    total_hamming_error
}

// Helper to get all outputs for logging
fn get_all_outputs(genome: &Genome) -> Vec<u8> {
    let mut results = Vec::new();
    for i in 0..PI_DIGITS.len() {
        let dp_input_bools = u8_to_bool_array(i as u8, NUM_DP_INPUT_BITS);
        let outputs = genome.eval(&dp_input_bools);
        results.push(bools_to_u8(&outputs));
    }
    results
}

fn u8_to_bool_array(n: u8, bits: usize) -> Vec<bool> {
    let mut arr = Vec::with_capacity(bits);
    for i in 0..bits {
        arr.push((n >> i) & 1 == 1);
    }
    arr
}

fn bools_to_u8(bools: &[bool]) -> u8 {
    let mut val = 0u8;
    for (i, &b) in bools.iter().enumerate().take(8) { // Max 8 bits for u8
        if b { val |= 1 << i; }
    }
    val
}

fn save_dot(genome: &Genome, filename: &str) {
    use std::io::Write;
    let mut file = std::fs::File::create(filename).unwrap();
    writeln!(file, "digraph Circuit {{ ").unwrap();
    writeln!(file, "  rankdir=LR;").unwrap();
    writeln!(file, "  node [fontname=\"Monaco\"];").unwrap();
    
    // Draw Inputs
    for i in 0..genome.num_inputs {
        // Corrected: Use {} to format variable 'i'
        writeln!(file, "  in_{} [label=\"IN_{}\", shape=box, style=filled, fillcolor=lightgrey];", i, i).unwrap();
    }
    
    // Draw Nodes
    for (i, node) in genome.nodes.iter().enumerate() {
        let idx = genome.num_inputs + i;
        let label = format!("{:?}", node.op);
        let color = match node.op {
            gate_multi::Operation::AND => "lightblue",
            gate_multi::Operation::OR => "lightgreen",
            gate_multi::Operation::XOR => "plum",
            gate_multi::Operation::NAND => "salmon",
            gate_multi::Operation::NOR => "lightyellow",
            gate_multi::Operation::NOT => "orange",
            gate_multi::Operation::WIRE => "white",
        };
        // Corrected: Use {} for idx, label, color
        writeln!(file, "  n_{} [label=\"{}\", shape=ellipse, style=filled, fillcolor=\"{}\"];", idx, label, color).unwrap();
        
        // Edges
        let output_node_name = format!("n_{}", idx);
        let input_node_name = |input_idx: usize| {
            // Corrected: Use {} for input_idx
            if input_idx < genome.num_inputs { format!("in_{}", input_idx) } else { format!("n_{}", input_idx) }
        };

        if node.op != gate_multi::Operation::NOT && node.op != gate_multi::Operation::WIRE {
             // Corrected: Pass formatted strings directly
             writeln!(file, "  {} -> {} [label=\"a\"];", input_node_name(node.in_a), output_node_name).unwrap();
             writeln!(file, "  {} -> {} [label=\"b\"];", input_node_name(node.in_b), output_node_name).unwrap();
        } else {
             // Corrected: Pass formatted string directly
             writeln!(file, "  {} -> {};", input_node_name(node.in_a), output_node_name).unwrap();
        }
    }
    
    // Draw Outputs
    for (i, &out_idx) in genome.outputs.iter().enumerate() {
        // Corrected: Use {} for 'i'
        writeln!(file, "  out_{} [label=\"OUT_{}\", shape=doublecircle];", i, i).unwrap();
        let src = if out_idx < genome.num_inputs { format!("in_{}", out_idx) } else { format!("n_{}", out_idx) };
        // Corrected: Pass formatted strings directly
        writeln!(file, "  {} -> out_{};", src, i).unwrap();
    }
    
    writeln!(file, "}}").unwrap();
    println!("Saved DOT to {}", filename);
    
    // Try to render PNG
    let output = std::process::Command::new("dot")
        .args(&["-Tpng", filename, "-o", &filename.replace(".dot", ".png")])
        .output();
        
    match output {
        Ok(_) => println!("Rendered PNG to {}", filename.replace(".dot", ".png")),
        Err(e) => println!("Failed to render PNG (is Graphviz installed?): {}", e),
    }
}
