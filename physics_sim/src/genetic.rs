use rand::Rng;

/// The DNA of our nano-machine.
/// 16 floating point numbers that define the geometry.
#[derive(Debug, Clone, Copy)]
pub struct Genome {
    pub genes: [f32; 16],
}

impl Genome {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        let mut genes = [0.0; 16];
        for i in 0..16 {
            genes[i] = rng.random(); // 0.0 to 1.0
        }
        Self { genes }
    }

    /// Mutate the genome slightly
    pub fn mutate(&self, rate: f32, strength: f32) -> Self {
        let mut rng = rand::rng();
        let mut new_genes = self.genes;
        for i in 0..16 {
            if rng.random::<f32>() < rate {
                new_genes[i] += (rng.random::<f32>() - 0.5) * strength;
                new_genes[i] = new_genes[i].clamp(0.0, 1.0);
            }
        }
        Self { genes: new_genes }
    }
    
    /// Crossover (Breeding) two genomes
    pub fn crossover(parent_a: &Self, parent_b: &Self) -> Self {
        let mut rng = rand::rng();
        let mut new_genes = [0.0; 16];
        let split_point = rng.random_range(1..15);
        
        for i in 0..16 {
            if i < split_point {
                new_genes[i] = parent_a.genes[i];
            } else {
                new_genes[i] = parent_b.genes[i];
            }
        }
        Self { genes: new_genes }
    }
}
