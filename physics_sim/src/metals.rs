#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetalType {
    Titanium,
    Aluminum,
    Iron,
    Tin,
    Vanadium, // Common Ti alloy partner
}

impl MetalType {
    pub fn atomic_mass(&self) -> f32 {
        match self {
            MetalType::Titanium => 47.87,
            MetalType::Aluminum => 26.98,
            MetalType::Iron => 55.85,
            MetalType::Tin => 118.71,
            MetalType::Vanadium => 50.94,
        }
    }

    // Approximate atomic radius in Angstroms
    pub fn atomic_radius(&self) -> f32 {
        match self {
            MetalType::Titanium => 1.47,
            MetalType::Aluminum => 1.43,
            MetalType::Iron => 1.26,
            MetalType::Tin => 1.40,
            MetalType::Vanadium => 1.34,
        }
    }

    // Simplified bonding strength/potential depth (epsilon)
    pub fn bond_strength(&self) -> f32 {
        match self {
            MetalType::Titanium => 1.5, // Strong
            MetalType::Iron => 1.2,
            MetalType::Vanadium => 1.4,
            MetalType::Aluminum => 0.8, // Weaker
            MetalType::Tin => 0.6,      // Weak
        }
    }
    
    pub fn to_gpu_id(&self) -> f32 {
        match self {
            MetalType::Titanium => 0.0,
            MetalType::Aluminum => 1.0,
            MetalType::Iron => 2.0,
            MetalType::Tin => 3.0,
            MetalType::Vanadium => 4.0,
        }
    }
}
