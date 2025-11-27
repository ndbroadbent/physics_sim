mod gate;
mod inputs;
mod targets;
mod gpu_eval;

use gate::Genome;
use inputs::{PrecomputedInputs, IMAGE_WIDTH, IMAGE_HEIGHT, NUM_CHUNKS, COORD_BITS};
use targets::Target;
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
use gpu_eval::GpuEvaluator;

// CGP Parameters
const NUM_INPUTS: usize = COORD_BITS * 2 + 1;
const NUM_NODES: usize = 600;
const NUM_EPOCHS: usize = 10000; 
const POPULATION_SIZE: usize = 4096; 
const FRONTIER_SIZE: usize = 64;     
const MAX_STAGNATION: usize = 20;    

#[derive(Clone)]
struct Individual {
    genome: Genome,
    fitness: (u64, u64), // (Error, ActiveNodes)
    last_improved_epoch: usize,
}

struct CandidateMetadata {
    parent_fitness: (u64, u64),
    parent_last_improved_epoch: usize,
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

    /// Save individual image frames when accuracy improves
    #[arg(long, default_value_t = false)]
    save_images: bool,
}

fn main() {
    let args = Args::parse();
    let seed = args.seed.unwrap_or_else(|| rand::thread_rng().gen());
    println!("Initializing Genetic Logic Shapes (GPU Accelerated + Parsimony)...");
    println!("Seed: {}", seed);
    
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let inputs = PrecomputedInputs::new();
    let target_circle = Target::circle(80.0); 
    let target_square = Target::square(80.0); 
    let target_checkerboard = Target::checkerboard(16); 
    fs::create_dir_all("evolution_output").unwrap();

    let font_path = "/System/Library/Fonts/Monaco.ttf";
    let font_data = fs::read(font_path).expect("Failed to load font");
    let font = Font::try_from_bytes(&font_data).expect("Error constructing Font");

    // GPU Setup
    println!("Setting up GPU...");
    let mut evaluator = pollster::block_on(GpuEvaluator::new(&inputs, &target_checkerboard));
    println!("GPU Ready.");

    let mut ffmpeg_stdin: Option<ChildStdin> = if args.video {
        let mut ffmpeg = Command::new("ffmpeg")
            .args(&[
                "-y", "-f", "rawvideo", "-pixel_format", "rgb24",
                "-video_size", &format!("{}"x"{}", IMAGE_WIDTH, IMAGE_HEIGHT),
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
    let mut population: Vec<Genome> = (0..FRONTIER_SIZE)
        .map(|_| Genome::new_random(NUM_INPUTS, NUM_NODES, &mut rng))
        .collect();
    
    let fitness_errors = evaluator.evaluate_batch(&population);
    
    let mut frontier: Vec<Individual> = population.into_iter().zip(fitness_errors).map(|(g, err)| {
        let active = g.active_node_count() as u64;
        Individual { genome: g, fitness: (err, active), last_improved_epoch: 0 }
    }).collect();

    frontier.sort_by(|a, b| a.fitness.cmp(&b.fitness));
    
    let mut best_ever_individual = frontier[0].clone();
    let mut last_saved_accuracy = 0.0;
    let mut epochs_since_global_improvement = 0;
    
    let mut hall_of_fame: Vec<Genome> = Vec::new();

    for epoch in 0..NUM_EPOCHS {
        // Asteroid Check
        if epochs_since_global_improvement > 30 {
            println!("☄️  ASTEROID IMPACT! Global stagnation detected. Rebuilding from the Scrapyard. ☄️");
            
            // 1. Add King to Hall of Fame (Compacted)
            let compacted_king = best_ever_individual.genome.compact();
            hall_of_fame.push(compacted_king);
            
            println!("Hall of Fame Size: {}", hall_of_fame.len());

            frontier.clear();
            
            // 2. Fill Population
            // A. 20% Fresh Random (Exploration)
            let random_quota = FRONTIER_SIZE / 5;
            
            for _ in 0..random_quota {
                let g = Genome::new_random(NUM_INPUTS, NUM_NODES, &mut rng);
                frontier.push(Individual { genome: g, fitness: (u64::MAX, u64::MAX), last_improved_epoch: epoch });
            }
            
            // B. 80% The Genetic Blender (Scrap Yard 2.0)
            while frontier.len() < FRONTIER_SIZE {
                if hall_of_fame.is_empty() {
                    let g = Genome::new_random(NUM_INPUTS, NUM_NODES, &mut rng);
                    frontier.push(Individual { genome: g, fitness: (u64::MAX, u64::MAX), last_improved_epoch: epoch });
                    continue;
                }

                let idx_a = rng.gen_range(0..hall_of_fame.len());
                let idx_b = rng.gen_range(0..hall_of_fame.len());
                
                // Chop
                let pieces_a = chop_genome(&hall_of_fame[idx_a], rng.gen_range(2..=4), &mut rng);
                let pieces_b = chop_genome(&hall_of_fame[idx_b], rng.gen_range(2..=4), &mut rng);

                // Mix
                let mut soup = Vec::new();
                soup.extend(pieces_a);
                soup.extend(pieces_b);
                
                // Shuffle chunks
                for i in (1..soup.len()).rev() {
                    let j = rng.gen_range(0..=i);
                    soup.swap(i, j);
                }

                // Reassemble
                let mut new_nodes = Vec::new();
                let total_soup_nodes: usize = soup.iter().map(|(p, _)| p.len()).sum();
                let remaining_space = NUM_NODES.saturating_sub(total_soup_nodes);
                let min_random = (NUM_NODES as f64 * 0.30) as usize;
                let random_fill_target = remaining_space.max(min_random).min(NUM_NODES);
                let nodes_per_gap = random_fill_target / (soup.len() + 1);

                for (chunk, original_start_offset) in soup {
                    // 1. Add random gap
                    for _ in 0..nodes_per_gap {
                        if new_nodes.len() >= NUM_NODES { break; }
                        let limit = NUM_INPUTS + new_nodes.len();
                        new_nodes.push(gate::Node {
                            op: gate::Operation::random(&mut rng),
                            in_a: rng.gen_range(0..limit),
                            in_b: rng.gen_range(0..limit),
                        });
                    }

                    // 2. Add Chunk
                    let new_chunk_start = new_nodes.len();
                    let offset_delta = (new_chunk_start as isize) - (original_start_offset as isize);

                    for (_i, old_node) in chunk.iter().enumerate() {
                        if new_nodes.len() >= NUM_NODES { break; }
                        
                        let mut new_node = old_node.clone();
                        let current_abs_idx = NUM_INPUTS + new_nodes.len();

                        // Fix Input A
                        if new_node.in_a >= NUM_INPUTS {
                            let remapped = (new_node.in_a as isize + offset_delta) as usize;
                            // Check validity: must point to something before us
                            if remapped < current_abs_idx && remapped >= NUM_INPUTS {
                                new_node.in_a = remapped;
                            } else {
                                // Broken link (pointed outside chunk or future), rewire randomly
                                new_node.in_a = rng.gen_range(0..current_abs_idx);
                            }
                        }
                        
                        // Fix Input B
                        if new_node.in_b >= NUM_INPUTS {
                            let remapped = (new_node.in_b as isize + offset_delta) as usize;
                            if remapped < current_abs_idx && remapped >= NUM_INPUTS {
                                new_node.in_b = remapped;
                            } else {
                                new_node.in_b = rng.gen_range(0..current_abs_idx);
                            }
                        }
                        
                        new_nodes.push(new_node);
                    }
                }
                
                // Fill remaining
                while new_nodes.len() < NUM_NODES {
                     let limit = NUM_INPUTS + new_nodes.len();
                     new_nodes.push(gate::Node {
                        op: gate::Operation::random(&mut rng),
                        in_a: rng.gen_range(0..limit),
                        in_b: rng.gen_range(0..limit),
                     });
                }
                
                let chimera = Genome {
                    num_inputs: NUM_INPUTS,
                    nodes: new_nodes,
                    output_node_idx: rng.gen_range(0..NUM_INPUTS + NUM_NODES),
                };
                
                frontier.push(Individual { genome: chimera, fitness: (u64::MAX, u64::MAX), last_improved_epoch: epoch });
            }
            
            epochs_since_global_improvement = 0;
            last_saved_accuracy = 0.0; // Reset image saving ratchet
            
            best_ever_individual = frontier[0].clone(); // Reset King
        }

        // 1. Expand
        let children_per_parent = POPULATION_SIZE / frontier.len();
        
        let mut next_gen_genomes = Vec::with_capacity(POPULATION_SIZE);
        let mut next_gen_metadata = Vec::with_capacity(POPULATION_SIZE);

        // Keep elites
        for p in &frontier {
            next_gen_genomes.push(p.genome.clone());
            next_gen_metadata.push(CandidateMetadata {
                parent_fitness: p.fitness,
                parent_last_improved_epoch: p.last_improved_epoch,
            });
        }

        for parent in &frontier {
            let stagnation = epoch - parent.last_improved_epoch;
            
            for _ in 0..children_per_parent {
                let mut child = parent.genome.clone();
                
                let roll = rng.gen::<f64>();
                let rate = if stagnation < 3 {
                    if roll < 0.6 { 0.001 } else if roll < 0.9 { 0.005 } else { 0.02 }
                } else if stagnation < 7 {
                    if roll < 0.3 { 0.001 } else if roll < 0.7 { 0.005 } else { 0.02 }
                } else if stagnation < 12 {
                    if roll < 0.2 { 0.005 } else if roll < 0.6 { 0.02 } else { 0.05 }
                } else {
                    if roll < 0.3 { 0.02 } else if roll < 0.7 { 0.05 } else { 0.25 }
                };
                
                child.mutate(rate, &mut rng);
                next_gen_genomes.push(child);
                next_gen_metadata.push(CandidateMetadata {
                    parent_fitness: parent.fitness,
                    parent_last_improved_epoch: parent.last_improved_epoch,
                });
            }
        }

        // 2. GPU Eval
        let fitness_errors = evaluator.evaluate_batch(&next_gen_genomes);

        // 3. Process Results (Correctly Inherit Staleness)
        let mut candidates: Vec<Individual> = next_gen_genomes.into_iter()
            .zip(fitness_errors)
            .zip(next_gen_metadata)
            .map(|((g, err), meta)| {
                let active = g.active_node_count() as u64;
                let fit = (err, active);
                let last_improved = if fit < meta.parent_fitness {
                    epoch // Improved!
                } else {
                    meta.parent_last_improved_epoch // Inherit staleness
                };
                Individual { genome: g, fitness: fit, last_improved_epoch: last_improved }
            }).collect();

        // 4. Prune Stagnant Branches
        let before_count = candidates.len();
        candidates.retain(|ind| (epoch - ind.last_improved_epoch) <= MAX_STAGNATION);
        let pruned_count = before_count - candidates.len();

        // 5. Sort & Truncate
        candidates.sort_by(|a, b| a.fitness.cmp(&b.fitness));
        candidates.dedup_by(|a, b| a.fitness == b.fitness);
        candidates.truncate(FRONTIER_SIZE);
        
        frontier = candidates;

        // 6. Refill if low
        while frontier.len() < FRONTIER_SIZE {
             let g = Genome::new_random(NUM_INPUTS, NUM_NODES, &mut rng);
             frontier.push(Individual { genome: g, fitness: (u64::MAX, u64::MAX), last_improved_epoch: epoch });
        }
        frontier.sort_by(|a, b| a.fitness.cmp(&b.fitness));

        // Update Global Best
        if frontier[0].fitness < best_ever_individual.fitness {
            // Only reset asteroid timer if ERROR improved (ignore node count optimization)
            if frontier[0].fitness.0 < best_ever_individual.fitness.0 {
                epochs_since_global_improvement = 0;
            } else {
                epochs_since_global_improvement += 1;
            }
            best_ever_individual = frontier[0].clone();
        } else {
            epochs_since_global_improvement += 1;
        }

        // Break if perfect solution found (0 errors)
        if best_ever_individual.fitness.0 == 0 {
            println!("SOLVED at Epoch {}! Fitness: (0, {}) (100% accurate)", epoch, best_ever_individual.fitness.1);
            save_dot(&best_ever_individual.genome, "checkerboard_perfect.dot");
            break; 
        }

        let current_accuracy = 1.0 - (frontier[0].fitness.0 as f64 / inputs::TOTAL_PIXELS as f64);
        
        if epoch % 1 == 0 { // Reduced logging frequency
             println!("Epoch {:04} | Fitness: {:8} | Nodes: {:3} | Acc: {:.2}% | Stale: {:2} | Pruned: {:4}", 
                epoch, 
                frontier[0].fitness.0, 
                frontier[0].fitness.1,
                current_accuracy * 100.0, 
                epoch - frontier[0].last_improved_epoch,
                pruned_count);
        }

        // Save Image on Improvement
        if args.save_images {
            let best_acc = 1.0 - (best_ever_individual.fitness.0 as f64 / inputs::TOTAL_PIXELS as f64);
            
            let threshold = if best_acc < 0.90 { 0.05 } else { 0.01 };
            
            if best_acc > last_saved_accuracy + threshold {
                let acc_str = (best_acc * 10000.0).round() as u32;
                let filename = format!("evolution_output/gen_{:05}_acc_{:04}.png", epoch, acc_str);
                save_diff_image(&best_ever_individual.genome, &inputs, 0u64, &target_checkerboard, &filename);
                last_saved_accuracy = best_acc;
                println!("Saved improvement: {}", filename);
            }
        }

        // Video Output
        if let Some(ref mut stdin) = ffmpeg_stdin {
             if epoch % 1 == 0 { // Reduced logging frequency
                let frame = render_frame(&frontier[0].genome, &inputs, &target_checkerboard, epoch, frontier[0].fitness.0, &font);
                stdin.write_all(&frame).unwrap();
             }
        }
    }
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
            if global_idx >= inputs::TOTAL_PIXELS { break; } // Corrected from >= to > for off-by-one
            
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
            if global_idx >= inputs::TOTAL_PIXELS { break; } // Corrected from >= to > for off-by-one
            
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

fn save_ground_truth(target: &Target, filename: &str) {
    let mut img = ImageBuffer::new(IMAGE_WIDTH, IMAGE_HEIGHT);
    for i in 0..NUM_CHUNKS {
        let expected_chunk = target.expected_output[i];
        for bit in 0..inputs::CHUNK_SIZE {
            let global_idx = i * inputs::CHUNK_SIZE + bit;
            if global_idx >= inputs::TOTAL_PIXELS { break; } // Corrected from >= to > for off-by-one
            let x = (global_idx as u32) % IMAGE_WIDTH;
            let y = (global_idx as u32) / IMAGE_WIDTH;
            let val: u8 = if (expected_chunk >> bit) & 1 == 1 { 0 } else { 255 };
            img.put_pixel(x, y, Luma([val]));
        }
    }
    img.save(filename).unwrap();
}

fn chop_genome(genome: &Genome, num_pieces: usize, rng: &mut impl Rng) -> Vec<(Vec<gate::Node>, usize)> {
    // 1. Compact to get the functional core
    let compacted = genome.compact();
    let total_nodes = compacted.nodes.len();
    
    if total_nodes < num_pieces {
        return vec![(compacted.nodes, 0)];
    }

    // 2. Generate split points
    let mut cuts = Vec::new();
    for _ in 0..num_pieces - 1 {
        cuts.push(rng.gen_range(1..total_nodes));
    }
    cuts.sort();
    cuts.dedup();
    
    // 3. Slice
    let mut pieces = Vec::new();
    let mut current_start = 0;
    
    for cut in cuts {
        if cut > current_start {
            let chunk = compacted.nodes[current_start..cut].to_vec();
            pieces.push((chunk, current_start));
            current_start = cut;
        }
    }
    // Last piece
    if current_start < total_nodes {
        let chunk = compacted.nodes[current_start..].to_vec();
        pieces.push((chunk, current_start));
    }
    
    pieces
}

fn save_dot(genome: &Genome, filename: &str) {
    use std::io::Write;
    let mut file = std::fs::File::create(filename).unwrap();
    writeln!(file, "digraph Circuit {{").unwrap(); 
    writeln!(file, "  rankdir=LR;").unwrap();
    writeln!(file, "  node [fontname=\"Monaco\"];").unwrap();
    
    for i in 0..genome.num_inputs {
        let label = if i < 8 { format!("X_{{}}", i) } else if i < 16 { format!("Y_{{}}", i - 8) } else { "ShapeID".to_string() };
        writeln!(file, "  in_{{}} [label=\"{{}}\", shape=box, style=filled, fillcolor=lightgrey];", i, label).unwrap();
    }
    
    for (i, node) in genome.nodes.iter().enumerate() {
        let idx = genome.num_inputs + i;
        let label = format!("{:?}", node.op);
        let color = match node.op {
            gate::Operation::AND => "lightblue",
            gate::Operation::OR => "lightgreen",
            gate::Operation::XOR => "plum",
            gate::Operation::NAND => "salmon",
            gate::Operation::NOR => "lightyellow",
            gate::Operation::NOT => "orange",
            gate::Operation::WIRE => "white",
        };
        writeln!(file, "  n_{{}} [label=\"{{}}\", shape=ellipse, style=filled, fillcolor=\"{{}}\"];", idx, label, color).unwrap();
        
        let output_node_name = format!("n_{{}}", idx);
        let input_node_name = |input_idx: usize| {
            if input_idx < genome.num_inputs { format!("in_{{}}", input_idx) } else { format!("n_{{}}", input_idx) }
        };

        if node.op != gate::Operation::NOT && node.op != gate::Operation::WIRE {
             writeln!(file, "  {{}} -> {{}} [label=\"a\"];", input_node_name(node.in_a), output_node_name).unwrap();
             writeln!(file, "  {{}} -> {{}} [label=\"b\"];", input_node_name(node.in_b), output_node_name).unwrap();
        } else {
             writeln!(file, "  {{}} -> {{}}", input_node_name(node.in_a), output_node_name).unwrap();
        }
    }
    
    let out_idx = genome.output_node_idx;
    writeln!(file, "  out_0 [label=\"Pixel\", shape=doublecircle];").unwrap();
    let src = if out_idx < genome.num_inputs { format!("in_{{}}", out_idx) } else { format!("n_{{}}", out_idx) };
    writeln!(file, "  {{}} -> out_0;", src).unwrap();
    
    writeln!(file, "}}").unwrap();
    println!("Saved DOT to {}", filename);
    
    let output = std::process::Command::new("dot")
        .args(&["-Tpng", filename, "-o", &filename.replace(".dot", ".png")])
        .output();
        
    match output {
        Ok(_) => println!("Rendered PNG to {}", filename.replace(".dot", ".png")),
        Err(e) => println!("Failed to render PNG (is Graphviz installed?): {}", e),
    }
}