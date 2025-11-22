use crate::elements::{Element, MaterialDef};
use std::fmt;
use plotters::prelude::*; // Import plotters for convenience

#[derive(Debug, Clone)]
pub struct ExperimentConfig {
    pub name: String,
    pub host_material: MaterialDef,
    pub n_dopant: Option<Element>, // None for no N-doping
    pub p_dopant: Option<Element>, // None for no P-doping
    pub n_doping_concentration: f64,
    pub p_doping_concentration: f64,
    pub photon_energy_ev: f64,
    pub photon_count: usize,
    pub sim_steps: u32,
    pub dt: f32,
    pub sim_width: u32,
    pub sim_height: u32,
    pub atom_density: f64, // Changed from f32 to f64 for consistency
}

impl fmt::Display for ExperimentConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: Host={} ({:.2}eV), N={:?}({:.2}%), P={:?}({:.2}%)",
            self.name,
            self.host_material.name,
            self.host_material.band_gap,
            self.n_dopant.map(|e| format!("{:?}", e)).unwrap_or_else(|| "None".to_string()),
            self.n_doping_concentration * 100.0,
            self.p_dopant.map(|e| format!("{:?}", e)).unwrap_or_else(|| "None".to_string()),
            self.p_doping_concentration * 100.0,
        )
    }
}


#[derive(Debug, Clone)]
pub struct ExperimentResult {
    pub config_name: String,
    pub material_band_gap: f64,
    pub estimated_voc: f64,
    pub collected_pairs: f64,
    pub efficiency_percent: f64,
    pub absorbed_photons: u32,
    pub transmitted_photons: u32,
}

pub async fn run_gpu_experiment(
    config: &ExperimentConfig,
    _device: &wgpu::Device, // Marked as unused, as the GPU setup for the actual sim is done once in main
    _queue: &wgpu::Queue,   // Marked as unused
) -> ExperimentResult {
    // This will eventually contain the logic from main_gpu.rs
    // For now, let's return dummy data.
    
    // Placeholder calculation based on some assumptions
    let absorbed_photons = (config.photon_count as f64 * config.atom_density * 0.5) as u32;
    let transmitted_photons = config.photon_count as u32 - absorbed_photons;
    
    let collected_pairs = absorbed_photons as f64 * 0.8;
    let estimated_voc = config.host_material.band_gap * 0.7;
    let output_energy_proxy_ev = collected_pairs * estimated_voc;
    let total_input_photon_energy_ev = config.photon_count as f64 * config.photon_energy_ev;
    let efficiency_percent = (output_energy_proxy_ev / total_input_photon_energy_ev) * 100.0;

    ExperimentResult {
        config_name: config.name.clone(),
        material_band_gap: config.host_material.band_gap,
        estimated_voc,
        collected_pairs,
        efficiency_percent,
        absorbed_photons,
        transmitted_photons,
    }
}

pub fn plot_results(results: &[ExperimentResult], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(filename, (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_efficiency = results.iter().map(|r| r.efficiency_percent).fold(0.0, f64::max);
    
    let num_experiments = results.len();

    let mut chart = ChartBuilder::on(&root)
        .caption("Solar Panel Efficiency Comparison", ("sans-serif", 50).into_font())
        .margin(10)
        .x_label_area_size(80)
        .y_label_area_size(80)
        .build_cartesian_2d(
            -0.5..(num_experiments as f64 - 0.5), // X-axis range to center bars
            0.0..max_efficiency * 1.2, // Y-axis for efficiency
        )?;

    chart.configure_mesh()
        .x_desc("Experiment")
        .y_desc("Efficiency (%)")
        .x_label_formatter(&|x| {
            // Map floating point x back to index and get label
            let idx = (x + 0.5).round() as usize; // Adjust for centering
            if idx < num_experiments {
                results[idx].config_name.clone()
            } else {
                "".to_string()
            }
        })
        .y_label_formatter(&|y| format!("{:.2}%", y))
        .draw()?;

    chart.draw_series(
        results.iter().enumerate().map(|(idx, r)| {
            let x_coord = idx as f64;
            Rectangle::new([(x_coord - 0.4, 0.0), (x_coord + 0.4, r.efficiency_percent)], BLUE.filled())
        })
    )?;

    Ok(())
}

