#[path = "../gate_universal.rs"]
mod gate_universal;

use gate_universal::Genome;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand::Rng;
use rayon::prelude::*;

const NUM_NODES: usize = 50;
const INPUT_BITS: usize = 11;
const FRONTIER_SIZE: usize = 64; // Independent lineages
const CHILDREN_PER_PARENT: usize = 10; 
const MAX_STAGNATION: usize = 50;

#[derive(Clone)]
struct Individual {
    genome: Genome,
    fitness: u32, // Error count
    target_len: usize, // To calculate accuracy
    stagnation: usize,
}

fn main() {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    
    // Initialize Frontier with random genomes
    let mut frontier: Vec<Individual> = (0..FRONTIER_SIZE).map(|_| {
        let g = Genome::new_random(INPUT_BITS, NUM_NODES, &mut rng);
        let (fit, len) = evaluate(&g);
        Individual { genome: g, fitness: fit, target_len: len, stagnation: 0 }
    }).collect();

    let mut gen = 0;
    let mut best_global_acc = 0.0;

    loop {
        // 1. Expand (Generate Children) - Serial RNG part
        // We flatten the list of tasks: (ParentIndex, ChildGenome)
        let mut tasks = Vec::with_capacity(FRONTIER_SIZE * CHILDREN_PER_PARENT);
        
        for (p_idx, parent) in frontier.iter().enumerate() {
            // Keep parent as a candidate (Elitism/Stasis)
            tasks.push((p_idx, parent.genome.clone(), true)); 
            
            for _ in 0..CHILDREN_PER_PARENT {
                let mut child = parent.genome.clone();
                // Adaptive mutation based on parent stagnation?
                let rate = if parent.stagnation > 20 { 0.10 } else { 0.02 };
                child.mutate(rate, &mut rng);
                tasks.push((p_idx, child, false));
            }
        }

        // 2. Parallel Evaluation
        let results: Vec<(usize, Individual)> = tasks.into_par_iter().map(|(p_idx, genome, is_parent)| {
            let (fit, len) = evaluate(&genome);
            let stagnation = if is_parent { 0 } else { 0 }; // Placeholder, handled in selection
            (p_idx, Individual { genome, fitness: fit, target_len: len, stagnation })
        }).collect();

        // 3. Selection (Local Competition - Island Model)
        // We group results by p_idx and pick the best for each slot.
        // This maintains diversity by ensuring 64 distinct lineages survive.
        
        let mut new_frontier = Vec::with_capacity(FRONTIER_SIZE);
        // Grouping is implicit because p_idx goes 0..63
        
        // We need to re-organize results. Since they are parallel, order is scrambled.
        // Let's sort by p_idx temporarily? Or just use a vector of vectors.
        // Easier: Sort results by p_idx.
        let mut sorted_results = results;
        sorted_results.sort_by_key(|(p_idx, _)| *p_idx);
        
        let mut current_p_idx = 0;
        let mut best_for_parent = &frontier[0]; // Dummy init
        let mut best_fit_for_parent = u32::MAX;
        let mut found_improvement = false;

        // Helper to process a group
        for (p_idx, ind) in &sorted_results {
            if *p_idx != current_p_idx {
                // Finalize previous group
                let mut winner = best_for_parent.clone();
                if !found_improvement {
                    winner.stagnation = frontier[current_p_idx].stagnation + 1;
                } else {
                    winner.stagnation = 0;
                }
                
                // Kick if stagnant
                if winner.stagnation > MAX_STAGNATION {
                    // Hard Reset this island
                    // We need an RNG here... but we are inside a loop.
                    // Hack: Mark it as "dead" and refill later? 
                    // Or just leave it high stagnation, and we handle "Refill" step after this loop.
                }
                new_frontier.push(winner);
                
                // Start new group
                current_p_idx = *p_idx;
                best_fit_for_parent = u32::MAX;
                found_improvement = false;
            }
            
            // Check candidate
            // Fitness Logic: Lower is better.
            // We accept if Strictly Better.
            // Or if Equal but Shorter? (Implicitly handled if fitness is error count? No)
            // Let's prioritize Error Count.
            
            if ind.fitness < best_fit_for_parent {
                best_for_parent = ind;
                best_fit_for_parent = ind.fitness;
                if ind.fitness < frontier[*p_idx].fitness {
                    found_improvement = true;
                }
            }
        }
        // Finalize last group
        let mut winner = best_for_parent.clone();
        if !found_improvement {
            winner.stagnation = frontier[current_p_idx].stagnation + 1;
        } else {
            winner.stagnation = 0;
        }
        new_frontier.push(winner);
        
        frontier = new_frontier;

        // 4. Refill / Global Processing
        // Calculate global stats
        let mut global_best_idx = 0;
        let mut min_error = u32::MAX;
        
        for (i, ind) in frontier.iter_mut().enumerate() {
            // Reset Logic
            if ind.stagnation > MAX_STAGNATION {
                // Reseed this island
                ind.genome = Genome::new_random(INPUT_BITS, NUM_NODES, &mut rng);
                let (f, l) = evaluate(&ind.genome);
                ind.fitness = f;
                ind.target_len = l;
                ind.stagnation = 0;
            }
            
            if ind.fitness < min_error {
                min_error = ind.fitness;
                global_best_idx = i;
            }
        }
        
        let best_ind = &frontier[global_best_idx];
        let acc = 1.0 - (best_ind.fitness as f64 / best_ind.target_len as f64);
        
        if acc > best_global_acc {
            best_global_acc = acc;
        }

        if gen % 10 == 0 {
            let active_nodes = best_ind.genome.get_active_genome().nodes.len();
            println!("Gen {:04} | Global Best: Err {} / {} | Acc: {:.2}% | Active: {}", 
                gen, best_ind.fitness, best_ind.target_len, acc * 100.0, active_nodes);
        }
        
        if min_error == 0 {
            println!("QUINE SOLVED at Gen {}! (Island {})", gen, global_best_idx);
            break;
        }
        
        gen += 1;
    }
}

fn evaluate(genome: &Genome) -> (u32, usize) {
    let active_genome = genome.get_active_genome();
    
    // Penalize genomes with too few active nodes (Minimum 20)
    if active_genome.nodes.len() < 20 { 
        return (u32::MAX, 1); 
    }

    let target = active_genome.to_bits();
    if target.is_empty() {
        return (u32::MAX, 1);
    }
    
    let mut error = 0;
    let mut input_bools = vec![false; INPUT_BITS];
    
    for (i, &expected_bit) in target.iter().enumerate() {
        for b in 0..INPUT_BITS {
            input_bools[b] = (i >> b) & 1 == 1;
        }
        
        let output = genome.eval(&input_bools);
        if output != expected_bit {
            error += 1;
        }
    }
    
    (error, target.len())
}