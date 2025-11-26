mod gate;
mod inputs;
mod targets;

use gate::Genome;
use inputs::{PrecomputedInputs, IMAGE_WIDTH, IMAGE_HEIGHT, NUM_CHUNKS, COORD_BITS};
use targets::Target;
use rayon::prelude::*;
use std::process::{Command, Stdio, ChildStdin};
use std::io::Write;
use std::fs;
use image::{ImageBuffer, Luma, Rgb};
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};
use clap::Parser;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand::Rng;

// CGP Parameters
const NUM_INPUTS: usize = COORD_BITS * 2 + 1;
const NUM_NODES: usize = 600;
const NUM_EPOCHS: usize = 10000; 
const EPOCH_GENS: usize = 50;   
const POPULATION_SIZE: usize = 64; 
const FRONTIER_SIZE: usize = 20; // Keep more potential branches
const MAX_STAGNATION: usize = 15; // Kill a branch if it doesn't improve for 15 epochs

#[derive(Clone)]
struct Individual {
    genome: Genome,
    fitness: u64,
    last_improved_epoch: usize,
    id: u64, // Just for tracking lineage/debugging
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Random seed (u64). If not provided, one will be generated.
    #[arg(long)]
    seed: Option<u64>,

    /// Enable video generation (ffmpeg required)
    #[arg(long, default_value_t = false)]
    video: bool,
}

fn main() {
    let args = Args::parse();

    // Deterministic RNG Setup
    let seed = args.seed.unwrap_or_else(|| rand::thread_rng().gen());
    println!("Initializing Genetic Logic Shapes (Staleness Pruning)...");
    println!("Seed: {}", seed);
    
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    // 1. Setup Data
    let inputs = PrecomputedInputs::new();
    let target_square = Target::square(80.0);
    
    fs::create_dir_all("evolution_output").unwrap();

    let font_path = "/System/Library/Fonts/Monaco.ttf";
    let font_data = fs::read(font_path).expect("Failed to load font");
    let font = Font::try_from_bytes(&font_data).expect("Error constructing Font");

    let mut ffmpeg_stdin: Option<ChildStdin> = if args.video {
        let mut ffmpeg = Command::new("ffmpeg")
            .args(&[
                "-y", "-f", "rawvideo", "-pixel_format", "rgb24",
                "-video_size", &format!("{}x{}", IMAGE_WIDTH, IMAGE_HEIGHT),
                "-framerate", "30", "-i", "-",
                "-c:v", "libx264", "-preset", "medium", "-crf", "18", "-pix_fmt", "yuv420p",
                "evolution.mp4"
            ])
            .stdin(Stdio::piped())
            .spawn()
            .expect("Failed to start ffmpeg");
        Some(ffmpeg.stdin.take().expect("Failed to open ffmpeg stdin"))
    } else {
        None
    };

    // Initialize Frontier
    let mut frontier: Vec<Individual> = (0..FRONTIER_SIZE)
        .map(|i| {
            let g = Genome::new_random(NUM_INPUTS, NUM_NODES, &mut rng);
            let fit = evaluate_genome(&g, &inputs, &target_square);
            Individual {
                genome: g,
                fitness: fit,
                last_improved_epoch: 0,
                id: i as u64,
            }
        })
        .collect();
    
    // Track Global Best separately for logging/video (even if it gets pruned from frontier)
    let mut best_ever_individual = frontier[0].clone();
    let mut last_saved_accuracy = 0.0;
    let mut next_id = FRONTIER_SIZE as u64;

    for epoch in 0..NUM_EPOCHS {
        // 1. Expand
        let tasks_per_parent = (POPULATION_SIZE / frontier.len()).max(1);
        
        // Create tasks
        let mut tasks = Vec::new();
        for parent in &frontier {
            for _ in 0..tasks_per_parent {
                let seed: u64 = rng.gen();
                // Variable mutation rates
                let mutation_rate = if rng.gen_bool(0.3) { 0.01 } else { 0.002 };
                tasks.push((parent.clone(), seed, mutation_rate, epoch));
            }
        }

        // Parallel Burst
        let new_candidates: Vec<Individual> = tasks.into_par_iter()
            .map(|(start_ind, seed, mut_rate, current_epoch)| {
                run_evolution_burst(start_ind, seed, mut_rate, EPOCH_GENS, current_epoch, &inputs, &target_square)
            })
            .collect();

        // 2. Merge
        frontier.extend(new_candidates);

        // 3. Prune Stagnant Branches (The Grim Reaper)
        let before_count = frontier.len();
        frontier.retain(|ind| (epoch - ind.last_improved_epoch) <= MAX_STAGNATION);
        let pruned_count = before_count - frontier.len();

        // 4. Sort and Deduplicate
        frontier.sort_by(|a, b| a.fitness.cmp(&b.fitness));
        frontier.dedup_by(|a, b| a.fitness == b.fitness); // Simple dedup by fitness
        frontier.truncate(FRONTIER_SIZE);

        // 5. Refill if empty or low (Diversity Injection)
        while frontier.len() < FRONTIER_SIZE {
            let g = Genome::new_random(NUM_INPUTS, NUM_NODES, &mut rng);
            let fit = evaluate_genome(&g, &inputs, &target_square);
            frontier.push(Individual {
                genome: g,
                fitness: fit,
                last_improved_epoch: epoch, // Fresh start
                id: next_id,
            });
            next_id += 1;
        }
        
        // Re-sort after refill
        frontier.sort_by(|a, b| a.fitness.cmp(&b.fitness));

        // Update Global Best (for history)
        if frontier[0].fitness < best_ever_individual.fitness {
            best_ever_individual = frontier[0].clone();
        }

        let current_accuracy = 1.0 - (frontier[0].fitness as f64 / inputs::TOTAL_PIXELS as f64);
        
        if epoch % 1 == 0 {
             println!("Epoch {:04} | Fitness: {:8} | Acc: {:.2}% | Stale: {:2} | Pruned: {:2} | Active: {}", 
                epoch, 
                frontier[0].fitness, 
                current_accuracy * 100.0, 
                epoch - frontier[0].last_improved_epoch,
                pruned_count,
                frontier[0].genome.active_node_count());
        }

        // Save Image on Improvement (using Global Best to see history, or Frontier Best to see current search?)
        // User probably wants to see the best thing found so far.
        let best_acc = 1.0 - (best_ever_individual.fitness as f64 / inputs::TOTAL_PIXELS as f64);
        if (best_acc - last_saved_accuracy).abs() > 0.0001 {
            let acc_str = (best_acc * 10000.0).round() as u32;
            let filename = format!("evolution_output/gen_{:05}_acc_{:04}.png", epoch * EPOCH_GENS, acc_str);
            save_diff_image(&best_ever_individual.genome, &inputs, u64::MAX, &target_square, &filename);
            last_saved_accuracy = best_acc;
            println!("Saved improvement: {}", filename);
        }

        // Video Output - Show the CURRENT frontier best, even if it's worse than global best, 
        // so we can see the "search" happening.
        if let Some(ref mut stdin) = ffmpeg_stdin {
             if epoch % 1 == 0 {
                // We render the Frontier[0] to show what the algo is currently working on
                let frame = render_frame(&frontier[0].genome, &inputs, &target_square, epoch * EPOCH_GENS, frontier[0].fitness, &font);
                stdin.write_all(&frame).unwrap();
             }
        }
    }
}

fn evaluate_genome(genome: &Genome, inputs: &PrecomputedInputs, target: &Target) -> u64 {
    let mut buffer = Vec::with_capacity(NUM_NODES + NUM_INPUTS);
    let mut total_errors = 0;
    for i in 0..NUM_CHUNKS {
        let mut input_vec = Vec::with_capacity(NUM_INPUTS);
        input_vec.extend_from_slice(&inputs.x_bits[i]);
        input_vec.extend_from_slice(&inputs.y_bits[i]);
        input_vec.push(u64::MAX); 

        let output = genome.eval(&input_vec, &mut buffer);
        let expected = target.expected_output[i];
        total_errors += (output ^ expected).count_ones() as u64;
    }
    total_errors
}

fn run_evolution_burst(mut parent: Individual, seed: u64, mutation_rate: f64, gens: usize, current_epoch: usize, inputs: &PrecomputedInputs, target: &Target) -> Individual {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    
    for _ in 0..gens {
        let mut child_genome = parent.genome.clone();
        child_genome.mutate(mutation_rate, &mut rng);
        let child_fit = evaluate_genome(&child_genome, inputs, target);
        
        if child_fit < parent.fitness {
            // Improvement!
            parent.genome = child_genome;
            parent.fitness = child_fit;
            parent.last_improved_epoch = current_epoch; // Reset staleness
        } 
        // If equal or worse, we ignore it in this simple burst model, 
        // OR we could allow neutral drift. 
        // Let's allow neutral drift but NOT update the timestamp.
        else if child_fit == parent.fitness {
             parent.genome = child_genome;
             // Do NOT update last_improved_epoch
        }
    }
    parent
}

fn render_frame(genome: &Genome, inputs: &PrecomputedInputs, target: &Target, gen: usize, fit: u64, font: &Font) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut img = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(IMAGE_WIDTH, IMAGE_HEIGHT);
    
    for i in 0..NUM_CHUNKS {
        let mut input_vec = Vec::with_capacity(NUM_INPUTS);
        input_vec.extend_from_slice(&inputs.x_bits[i]);
        input_vec.extend_from_slice(&inputs.y_bits[i]);
        input_vec.push(u64::MAX); 

        let output_chunk = genome.eval(&input_vec, &mut buffer);
        let expected_chunk = target.expected_output[i];

        for bit in 0..inputs::CHUNK_SIZE {
            let global_idx = i * inputs::CHUNK_SIZE + bit;
            if global_idx >= inputs::TOTAL_PIXELS { break; }
            
            let x = (global_idx as u32) % IMAGE_WIDTH;
            let y = (global_idx as u32) / IMAGE_WIDTH;

            let actual_val = (output_chunk >> bit) & 1;
            let expected_val = (expected_chunk >> bit) & 1;

            let color = match (actual_val, expected_val) {
                (1, 1) => Rgb([0u8, 255u8, 0u8]), 
                (1, 0) => Rgb([255u8, 0u8, 0u8]), 
                (0, 1) => Rgb([0u8, 0u8, 255u8]), 
                (0, 0) => Rgb([0u8, 0u8, 0u8]),   
                _ => Rgb([0u8, 0u8, 0u8]), 
            };
            img.put_pixel(x, y, color);
        }
    }
    
    let accuracy = 1.0 - (fit as f64 / inputs::TOTAL_PIXELS as f64);
    let info_text = format!("Gen: {:05} | Acc: {:.2}% | Active: {}", gen, accuracy * 100.0, genome.active_node_count());
    draw_text_mut(&mut img, Rgb([255, 255, 255]), 10, 10, Scale::uniform(20.0), font, &info_text);

    img.into_raw()
}

fn save_ground_truth(target: &Target, filename: &str) {
    let mut img = ImageBuffer::new(IMAGE_WIDTH, IMAGE_HEIGHT);
    for i in 0..NUM_CHUNKS {
        let expected_chunk = target.expected_output[i];
        for bit in 0..inputs::CHUNK_SIZE {
            let global_idx = i * inputs::CHUNK_SIZE + bit;
            if global_idx >= inputs::TOTAL_PIXELS { break; }
            let x = (global_idx as u32) % IMAGE_WIDTH;
            let y = (global_idx as u32) / IMAGE_WIDTH;
            let val: u8 = if (expected_chunk >> bit) & 1 == 1 { 0 } else { 255 };
            img.put_pixel(x, y, Luma([val]));
        }
    }
    img.save(filename).unwrap();
}

fn save_diff_image(genome: &Genome, inputs: &PrecomputedInputs, shape_id: u64, target: &Target, filename: &str) {
    let mut img = ImageBuffer::new(IMAGE_WIDTH, IMAGE_HEIGHT);
    let mut buffer = Vec::new();

    for i in 0..NUM_CHUNKS {
        let mut input_vec = Vec::with_capacity(NUM_INPUTS);
        input_vec.extend_from_slice(&inputs.x_bits[i]);
        input_vec.extend_from_slice(&inputs.y_bits[i]);
        input_vec.push(shape_id);

        let output_chunk = genome.eval(&input_vec, &mut buffer);
        let expected_chunk = target.expected_output[i];

        for bit in 0..inputs::CHUNK_SIZE {
            let global_idx = i * inputs::CHUNK_SIZE + bit;
            if global_idx >= inputs::TOTAL_PIXELS { break; }
            
            let x = (global_idx as u32) % IMAGE_WIDTH;
            let y = (global_idx as u32) / IMAGE_WIDTH;

            let actual_val = (output_chunk >> bit) & 1;
            let expected_val = (expected_chunk >> bit) & 1;

            let color = match (actual_val, expected_val) {
                (1, 1) => Rgb([0u8, 255u8, 0u8]), 
                (1, 0) => Rgb([255u8, 0u8, 0u8]), 
                (0, 1) => Rgb([0u8, 0u8, 255u8]), 
                (0, 0) => Rgb([0u8, 0u8, 0u8]),   
                _ => Rgb([0u8, 0u8, 0u8]), 
            };
            img.put_pixel(x, y, color);
        }
    }
    img.save(filename).unwrap();
}
