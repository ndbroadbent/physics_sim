use plotters::prelude::*;

/// The Analytical Gold Standard
/// Implements the expected physics formulas for comparison.
pub struct GoldStandard;

impl GoldStandard {
    /// Beer-Lambert Law: I(x) = I0 * e^(-n * sigma * x)
    /// n: density, sigma: cross_section, x: distance
    pub fn beer_lambert_intensity(i0: f64, n: f64, sigma: f64, x: f64) -> f64 {
        i0 * (-n * sigma * x).exp()
    }
    
    /// Generates a comparison plot between Simulation and Theory
    pub fn generate_attenuation_plot(
        filename: &str, 
        simulation_data: &[(f64, f64)], // (distance, count)
        n: f64, 
        sigma: f64,
        initial_count: f64
    ) -> Result<(), Box<dyn std::error::Error>> {
        
        let root = BitMapBackend::new(filename, (800, 600)).into_drawing_area();
        root.fill(&WHITE)?;

        let max_dist = simulation_data.iter().map(|(d, _)| *d).fold(0.0, f64::max);
        
        let mut chart = ChartBuilder::on(&root)
            .caption("Photon Attenuation: Sim vs Theory", ("sans-serif", 50).into_font())
            .margin(10)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(0.0..max_dist, 0.0..initial_count)?;

        chart.configure_mesh().draw()?;

        // Draw Theory Line
        chart.draw_series(LineSeries::new(
            (0..=(max_dist as i32 * 10)).map(|x| {
                let x_f = x as f64 / 10.0;
                (x_f, Self::beer_lambert_intensity(initial_count, n, sigma, x_f))
            }),
            &RED,
        ))?
        .label("Theory (Beer-Lambert)")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

        // Draw Simulation Points
        chart.draw_series(PointSeries::of_element(
            simulation_data.iter().copied(),
            5,
            &BLUE,
            &|c, s, st| {
                return EmptyElement::at(c)    // We want the point at (x,y)
                + Circle::new((0,0), s, st.filled()); // And a filled circle
            },
        ))?
        .label("Simulation")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

        chart.configure_series_labels()
            .background_style(&WHITE.mix(0.8))
            .border_style(&BLACK)
            .draw()?;

        Ok(())
    }
}
