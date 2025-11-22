use crate::elements::{Element, MaterialDef};
use std::fmt;
use plotters::prelude::*;

#[derive(Debug, Clone)]
pub struct LayerDef {
    pub material: MaterialDef,
    pub width: f32, // Width of this layer in simulation pixels
}

#[derive(Debug, Clone)]
pub struct ExperimentConfig {
    pub name: String,
    pub layers: Vec<LayerDef>, // Support multiple layers!
    pub photon_energy_ev: f64,
    pub photon_count: usize,
    pub sim_steps: u32,
    pub dt: f32,
    pub sim_width: u32,
    pub sim_height: u32,
    pub atom_density: f64,
}

impl fmt::Display for ExperimentConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{} Layers]", self.name, self.layers.len())
    }
}

#[derive(Debug, Clone)]
pub struct ExperimentResult {
    pub config_name: String,
    pub layers_info: String,
    pub total_efficiency_percent: f64,
    pub absorbed_counts: Vec<u32>, // Absorbed per layer
    pub transmitted_photons: u32,
}

// Helper to plot remains similar, but maybe show efficiency breakdown?
pub fn plot_results(results: &[ExperimentResult], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(filename, (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_efficiency = results.iter().map(|r| r.total_efficiency_percent).fold(0.0, f64::max);
    let num_experiments = results.len();

    let mut chart = ChartBuilder::on(&root)
        .caption("Multi-Junction Efficiency Comparison", ("sans-serif", 50).into_font())
        .margin(10)
        .x_label_area_size(80)
        .y_label_area_size(80)
        .build_cartesian_2d(
            -0.5..(num_experiments as f64 - 0.5),
            0.0..max_efficiency * 1.2,
        )?;

    chart.configure_mesh()
        .x_desc("Experiment")
        .y_desc("Total Efficiency (%)")
        .x_label_formatter(&|x| {
            let idx = (x + 0.5).round() as usize;
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
            Rectangle::new([(x_coord - 0.4, 0.0), (x_coord + 0.4, r.total_efficiency_percent)], BLUE.filled())
        })
    )?;

    Ok(())
}