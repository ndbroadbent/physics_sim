use physics_sim::shader_gen::SdfOp;
use std::fs;
use std::path::Path;

fn main() {
    println!("Initializing Genome Library...");
    let genes_dir = Path::new("genes");
    if !genes_dir.exists() {
        fs::create_dir(genes_dir).unwrap();
    }
    
    // Create a random genome
    let genome = SdfOp::random(4); // Depth 4
    let json = serde_json::to_string_pretty(&genome).unwrap();
    
    let filename = genes_dir.join("genome_test.json");
    fs::write(&filename, json).unwrap();
    
    println!("Saved test genome to {:?}", filename);
    
    // Generate WGSL
    let wgsl_body = genome.to_wgsl("p");
    println!("Generated WGSL Logic:\n{}", wgsl_body);
}

