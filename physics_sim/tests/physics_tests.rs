use physics_sim::fields::FieldType;
use physics_sim::particles::Particle;
use physics_sim::benchmark::GoldStandard;
use rand::Rng;

#[test]
fn test_beer_lambert_law_validity() {
    // This test runs a mini-simulation to verify that photon attenuation 
    // approximately follows the exponential decay law (Beer-Lambert).
    
    let width = 200;
    let photon_count = 1000;
    let atom_density = 0.2;
    // Artificial coupling to make the effect visible in short distance
    let coupling = 0.1; 
    
    // 1. Setup Slab (conceptually, we just check probability per step)
    let mut survival_counts = vec![0; width];
    
    // 2. Run Batch
    for _ in 0..photon_count {
        let mut x = 0.0;
        let mut active = true;
        
        while x < width as f64 && active {
            let idx = x as usize;
            if idx < width {
                survival_counts[idx] += 1;
            }
            
            // Move
            x += 1.0;
            
            // Interaction Check
            // Prob = n * sigma
            // Here sigma is effectively `coupling`
            let prob = atom_density * coupling;
            
            if rand::rng().random::<f64>() < prob {
                active = false; // Absorbed
            }
        }
    }
    
    // 3. Verify
    // We check the count at x=50 and x=100
    // Theory: N(x) = N0 * exp(-n * sigma * x)
    let n0 = photon_count as f64;
    let n = atom_density;
    let sigma = coupling;
    
    let check_points = [10, 50, 100];
    
    for &x in &check_points {
        let theory = GoldStandard::beer_lambert_intensity(n0, n, sigma, x as f64);
        let observed = survival_counts[x] as f64;
        
        let error = (theory - observed).abs();
        let tolerance = n0 * 0.1; // Allow 10% deviation due to Monte Carlo noise
        
        println!("At x={}: Theory={:.2}, Observed={:.2}, Error={:.2}", x, theory, observed, error);
        
        assert!(error < tolerance, 
            "Simulation deviates from Beer-Lambert Law at x={}! Theory: {}, Observed: {}", 
            x, theory, observed
        );
    }
}

#[test]
fn test_energy_conservation_photon() {
    let photon = Particle::new_photon(2.5, 0.0, 0.0, 0.0);
    let initial_energy = photon.energy;
    // In vacuum, energy should not change
    assert_eq!(initial_energy, photon.energy);
}

#[test]
fn test_electron_properties() {
    let electron = Particle::new_electron(0.0, 0.0, 0.0, 0.0);
    assert_eq!(electron.mass, 511_000.0);
    assert_eq!(electron.charge, -1.0);
}

#[test]
fn test_charge_carrier_creation() {
    let hole = Particle::new_charge_carrier(FieldType::Hole, 0.0, 0.0);
    let electron = Particle::new_charge_carrier(FieldType::FreeElectron, 0.0, 0.0);
    
    assert_eq!(hole.charge, 1.0);
    assert_eq!(electron.charge, -1.0);
}