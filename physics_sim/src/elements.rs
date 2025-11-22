#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Element {
    // Group 14 (Carbon family - Semiconductors)
    Carbon,     // Diamond/Graphene
    Silicon,    // Standard
    Germanium,  // Older transistors
    
    // Group 13 (Acceptors - P-type dopants)
    Boron,
    Aluminum,
    Gallium,
    Indium,

    // Group 15 (Donors - N-type dopants)
    Nitrogen,
    Phosphorus,
    Arsenic,
    Antimony,
    
    // Group 16 (Chalcogens - for CdTe etc)
    Oxygen,
    Sulfur,
    Selenium,
    Tellurium,

    // Group 12 (Transition metals - for CdTe)
    Cadmium,
    Zinc,
}

impl Element {
    pub fn atomic_number(&self) -> u32 {
        match self {
            Element::Boron => 5,
            Element::Carbon => 6,
            Element::Nitrogen => 7,
            Element::Oxygen => 8,
            Element::Aluminum => 13,
            Element::Silicon => 14,
            Element::Phosphorus => 15,
            Element::Sulfur => 16,
            Element::Zinc => 30,
            Element::Gallium => 31,
            Element::Germanium => 32,
            Element::Arsenic => 33,
            Element::Selenium => 34,
            Element::Cadmium => 48,
            Element::Indium => 49,
            Element::Antimony => 51,
            Element::Tellurium => 52,
        }
    }

    pub fn valence_electrons(&self) -> u8 {
        match self {
            Element::Boron | Element::Aluminum | Element::Gallium | Element::Indium => 3,
            Element::Carbon | Element::Silicon | Element::Germanium => 4,
            Element::Nitrogen | Element::Phosphorus | Element::Arsenic | Element::Antimony => 5,
            Element::Oxygen | Element::Sulfur | Element::Selenium | Element::Tellurium => 6,
            Element::Zinc | Element::Cadmium => 2, // Simplified for transition metals in semiconductor context
        }
    }

    /// Approximate Mass in atomic mass units (u)
    pub fn atomic_mass(&self) -> f64 {
        match self {
            Element::Silicon => 28.085,
            Element::Phosphorus => 30.974,
            Element::Boron => 10.81,
            // ... add others as needed for physics accuracy
            _ => 20.0, // Placeholder
        }
    }
}

#[derive(Debug, Clone)]
pub struct MaterialDef {
    pub name: String,
    pub band_gap: f64, // eV
    pub elements: Vec<(Element, f64)>, // (Element, Ratio)
}

impl MaterialDef {
    pub fn silicon() -> Self {
        Self {
            name: "Crystalline Silicon".to_string(),
            band_gap: 1.12,
            elements: vec![(Element::Silicon, 1.0)],
        }
    }

    pub fn germanium() -> Self {
        Self {
            name: "Germanium".to_string(),
            band_gap: 0.66,
            elements: vec![(Element::Germanium, 1.0)],
        }
    }

    pub fn gallium_arsenide() -> Self {
        Self {
            name: "Gallium Arsenide".to_string(),
            band_gap: 1.42,
            elements: vec![(Element::Gallium, 0.5), (Element::Arsenic, 0.5)],
        }
    }
    
    pub fn cadmium_telluride() -> Self {
         Self {
            name: "Cadmium Telluride".to_string(),
            band_gap: 1.44,
            elements: vec![(Element::Cadmium, 0.5), (Element::Tellurium, 0.5)],
        }
    }
}
