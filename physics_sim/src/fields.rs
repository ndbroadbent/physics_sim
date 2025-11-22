
/// Represents the strength/coupling of the fundamental forces.
/// The "Precision Slider" effectively scales these interactions.
/// If a field's coupling is 0.0, it is effectively disabled in the simulation.
#[derive(Debug, Clone, Copy)]
pub struct FieldCoupling {
    // --- Gauge Fields (Forces) ---
    pub electromagnetic: f64, // Photon field (alpha ~ 1/137)
    pub weak: f64,            // W+, W-, Z bosons
    pub strong: f64,          // Gluons

    // --- Scalar Field ---
    pub higgs: f64,           // Mass generation

    // --- Gravity (Optional, usually negligible for particle physics but good for completeness) ---
    pub gravity: f64,
}

impl FieldCoupling {
    /// Standard Model values (simplified/normalized for simulation)
    pub fn standard_model() -> Self {
        Self {
            electromagnetic: 1.0 / 137.0, // Fine-structure constant
            weak: 1e-6,                   // Fermi constant approximation (scaled)
            strong: 1.0,                  // Strong coupling constant (at low energy)
            higgs: 1.0,                   // Higgs vacuum expectation value scale
            gravity: 1e-40,               // Negligible
        }
    }

    /// A "Toy" universe where only EM exists (good for Solar Panel basics)
    pub fn electrodynamics_only() -> Self {
        Self {
            electromagnetic: 1.0 / 137.0,
            weak: 0.0,
            strong: 0.0,
            higgs: 0.0,   // Assuming mass is intrinsic for the toy model
            gravity: 0.0,
        }
    }
}

/// The 17 Fundamental Fields (Categorized)
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    // Gauge Bosons
    Photon,
    Gluon,
    WPlus,
    WMinus,
    ZBoson,
    
    // Leptons (Matter)
    Electron,
    Muon,
    Tau,
    ElectronNeutrino,
    MuonNeutrino,
    TauNeutrino,

    // Quarks (Matter)
    Up,
    Down,
    Charm,
    Strange,
    Top,
    Bottom,

    // Quasi-particles (Solid State)
    FreeElectron,
    Hole,

    // Scalar
    Higgs,
}
