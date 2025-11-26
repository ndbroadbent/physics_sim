#[path = "../gate_multi.rs"]
mod gate_multi;

use gate_multi::Genome;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand::Rng;

const TARGET: u8 = 42; // 00101010
const NUM_INPUTS: usize = 1; // Just a '1' constant
const NUM_NODES: usize = 50;
const NUM_OUTPUTS: usize = 8;
const POPULATION: usize = 100;

fn main() {
    let mut rng = ChaCha8Rng::seed_from_u64(12345);
    
    let mut parent = Genome::new_random(NUM_INPUTS, NUM_NODES, NUM_OUTPUTS, &mut rng);
    let mut gen = 0;
    
    loop {
        let mut best_child = parent.clone();
        let mut best_fitness = evaluate(&best_child);
        
        // 1+N Evolution Strategy
        for _ in 0..POPULATION {
            let mut child = parent.clone();
            child.mutate(0.05, &mut rng);
            let fitness = evaluate(&child);
            
            if fitness < best_fitness {
                best_child = child;
                best_fitness = fitness;
            }
        }
        
        parent = best_child;
        
        if gen % 100 == 0 {
            println!("Gen {:04} | Error: {} | Output: {:?}", gen, best_fitness, eval_to_u8(&parent));
        }
        
        if best_fitness == 0 {
            println!("SOLVED at Gen {}! Output: {}", gen, eval_to_u8(&parent));
            save_dot(&parent, "constant_42.dot");
            break;
        }
        
        gen += 1;
    }
}

fn evaluate(genome: &Genome) -> u32 {
    // Input is always [true] (power rail)
    let outputs = genome.eval(&[true]);
    let val = bools_to_u8(&outputs);
    (val ^ TARGET).count_ones() // Hamming distance
}

fn eval_to_u8(genome: &Genome) -> u8 {
    let outputs = genome.eval(&[true]);
    bools_to_u8(&outputs)
}

fn bools_to_u8(bools: &[bool]) -> u8 {
    let mut val = 0u8;
    for (i, &b) in bools.iter().enumerate().take(8) {
        if b { val |= 1 << i; }
    }
    val
}

fn save_dot(genome: &Genome, filename: &str) {
    use std::io::Write;
    let mut file = std::fs::File::create(filename).unwrap();
    writeln!(file, "digraph Circuit {{").unwrap();
    writeln!(file, "  rankdir=LR;").unwrap();
    writeln!(file, "  node [fontname=\"Monaco\"];").unwrap();
    
    // Draw Inputs
    for i in 0..genome.num_inputs {
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
        writeln!(file, "  n_{} [label=\"{}\", shape=ellipse, style=filled, fillcolor={}];", idx, label, color).unwrap();
        
        // Edges
        if node.op != gate_multi::Operation::NOT && node.op != gate_multi::Operation::WIRE {
             let src_a = if node.in_a < genome.num_inputs { format!("in_{}", node.in_a) } else { format!("n_{}", node.in_a) };
             let src_b = if node.in_b < genome.num_inputs { format!("in_{}", node.in_b) } else { format!("n_{}", node.in_b) };
             writeln!(file, "  {} -> n_{} [label=\"a\"];", src_a, idx).unwrap();
             writeln!(file, "  {} -> n_{} [label=\"b\"];", src_b, idx).unwrap();
        } else {
             // Unary
             let src_a = if node.in_a < genome.num_inputs { format!("in_{}", node.in_a) } else { format!("n_{}", node.in_a) };
             writeln!(file, "  {} -> n_{};", src_a, idx).unwrap();
        }
    }
    
    // Draw Outputs
    for (i, &out_idx) in genome.outputs.iter().enumerate() {
        writeln!(file, "  out_{} [label=\"OUT_{}\", shape=doublecircle];", i, i).unwrap();
        let src = if out_idx < genome.num_inputs { format!("in_{}", out_idx) } else { format!("n_{}", out_idx) };
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
