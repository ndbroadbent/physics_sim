use crate::fields::FieldType;
use crate::elements::Element;

#[derive(Debug, Clone)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone)]
pub struct Momentum {
    pub px: f64,
    pub py: f64,
    pub pz: f64,
}

#[derive(Debug, Clone)]
pub struct Particle {
    pub kind: FieldType,
    pub position: Position,
    pub momentum: Momentum,
    pub energy: f64,
    pub mass: f64, // In MeV/c^2
    pub charge: f64, // In elementary charge units (e)
    pub active: bool, // If false, removed from sim
}

impl Particle {
    pub fn new_photon(energy_ev: f64, x: f64, y: f64, z: f64) -> Self {
        Self {
            kind: FieldType::Photon,
            position: Position { x, y, z },
            momentum: Momentum { px: energy_ev, py: 0.0, pz: 0.0 }, 
            energy: energy_ev,
            mass: 0.0,
            charge: 0.0,
            active: true,
        }
    }

    pub fn new_electron(x: f64, y: f64, z: f64, bound_energy: f64) -> Self {
        Self {
            kind: FieldType::Electron,
            position: Position { x, y, z },
            momentum: Momentum { px: 0.0, py: 0.0, pz: 0.0 }, 
            energy: 511_000.0 - bound_energy, 
            mass: 511_000.0, 
            charge: -1.0,
            active: true,
        }
    }

    pub fn new_charge_carrier(kind: FieldType, x: f64, y: f64) -> Self {
        let (charge, mass) = match kind {
            FieldType::FreeElectron => (-1.0, 511_000.0),
            FieldType::Hole => (1.0, 511_000.0), // Effective mass approx
            _ => panic!("Not a carrier"),
        };
        
        Self {
            kind,
            position: Position { x, y, z: 0.0 },
            momentum: Momentum { px: 0.0, py: 0.0, pz: 0.0 },
            energy: 0.0, // Kinetic
            mass,
            charge,
            active: true,
        }
    }
}

/// Represents an Atom in the Lattice
pub struct Atom {
    pub element: Element,
    pub position: Position,
    pub electrons: Vec<Particle>,
}

impl Atom {
    pub fn new(element: Element, x: f64, y: f64, z: f64) -> Self {
        let mut electrons = Vec::new();
        
        // Populate valence electrons based on element properties
        // We assume inner shells are inert for this simulation
        let valence = element.valence_electrons();
        
        for _ in 0..valence {
            // Binding energy ~8-10 eV for semiconductors generally
            electrons.push(Particle::new_electron(x, y, z, 8.0)); 
        }

        Self {
            element,
            position: Position { x, y, z },
            electrons,
        }
    }
}
