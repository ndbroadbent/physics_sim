#[path = "../gate_multi.rs"]
mod gate_multi;

use gate_multi::Genome;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand::Rng;

// 4-bit addition: A (4 bits) + B (4 bits) = Result (5 bits, max 30)
const NUM_INPUT_BITS_A: usize = 4;
const NUM_INPUT_BITS_B: usize = 4;
const NUM_TOTAL_INPUTS: usize = NUM_INPUT_BITS_A + NUM_INPUT_BITS_B; // 8
const NUM_OUTPUT_BITS: usize = 5; // 4 bits sum + 1 bit carry

const NUM_NODES: usize = 300;
const POPULATION_SIZE: usize = 500;

fn main() {
    let mut rng = ChaCha8Rng::seed_from_u64(12345);
    
    let mut parent = Genome::new_random(NUM_TOTAL_INPUTS, NUM_NODES, NUM_OUTPUT_BITS, &mut rng);
    let mut gen = 0;
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
            // Kick: If stuck for too long, force move
            if stagnant_gens > 100 {
                println!("Kick! Hard Reset (stuck at fitness {})", evaluate(&parent));
                parent = Genome::new_random(NUM_TOTAL_INPUTS, NUM_NODES, NUM_OUTPUT_BITS, &mut rng);
                stagnant_gens = 0;
            }
        }
        
        if gen % 100 == 0 {
            println!("Gen {:04} | Error: {} | Rate: {:.2}", gen, best_fitness, mutation_rate);
        }
        
        if best_fitness == 0 {
            println!("SOLVED at Gen {}!", gen);
            
            // Verify a few cases
            verify_case(&parent, 3, 2);
            verify_case(&parent, 15, 1);
            verify_case(&parent, 15, 15);
            
            // DOT export removed for now to fix compilation
            break;
        }
        
        gen += 1;
    }
}

fn evaluate(genome: &Genome) -> u32 {
    let mut total_hamming_error = 0;
    
    // Iterate all 256 combinations
    for a in 0..(1 << NUM_INPUT_BITS_A) {
        for b in 0..(1 << NUM_INPUT_BITS_B) {
            let expected_sum = a + b;
            
            // Construct input bits: [A0, A1, A2, A3, B0, B1, B2, B3]
            let mut inputs = Vec::with_capacity(NUM_TOTAL_INPUTS);
            // A bits (Little Endian)
            for i in 0..NUM_INPUT_BITS_A { inputs.push((a >> i) & 1 == 1); }
            // B bits (Little Endian)
            for i in 0..NUM_INPUT_BITS_B { inputs.push((b >> i) & 1 == 1); }
            
            let outputs = genome.eval(&inputs);
            let val = bools_to_u8(&outputs);
            
            total_hamming_error += (val ^ (expected_sum as u8)).count_ones();
        }
    }
    total_hamming_error
}

fn verify_case(genome: &Genome, a: u8, b: u8) {
    let mut inputs = Vec::with_capacity(NUM_TOTAL_INPUTS);
    for i in 0..NUM_INPUT_BITS_A { inputs.push((a >> i) & 1 == 1); }
    for i in 0..NUM_INPUT_BITS_B { inputs.push((b >> i) & 1 == 1); }
    
    let outputs = genome.eval(&inputs);
    let val = bools_to_u8(&outputs);
    println!("  {} + {} = {} (Expected {})", a, b, val, a + b);
}

fn bools_to_u8(bools: &[bool]) -> u8 {
    let mut val = 0u8;
    for (i, &b) in bools.iter().enumerate().take(8) {
        if b { val |= 1 << i; }
    }
    val
}