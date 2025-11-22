use serde::{Serialize, Deserialize};
use rand::Rng;

// Better AST for SDF Generation
// An SDF is a function f(p) -> d
// We need nodes that return 'p' (vec3) and nodes that return 'd' (f32).

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SdfOp {
    Box([f32; 3]),
    Sphere(f32),
    Union(Box<SdfOp>, Box<SdfOp>),
    Subtract(Box<SdfOp>, Box<SdfOp>), // max(a, -b)
    Transform(Box<TransformOp>, Box<SdfOp>), // Apply transform to p before eval
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformOp {
    RotateY(f32),
    Translate([f32; 3]),
    Scale(f32),
    Repeat(f32),
}

impl SdfOp {
    pub fn random(depth: u32) -> Self {
        let mut rng = rand::rng();
        if depth == 0 || rng.random_bool(0.3) {
            if rng.random_bool(0.5) {
                SdfOp::Box([rng.random_range(0.5..2.0), rng.random_range(0.5..2.0), rng.random_range(0.5..2.0)])
            } else {
                SdfOp::Sphere(rng.random_range(0.5..2.0))
            }
        } else {
            match rng.random_range(0..4) {
                0 => SdfOp::Union(Box::new(SdfOp::random(depth - 1)), Box::new(SdfOp::random(depth - 1))),
                1 => SdfOp::Subtract(Box::new(SdfOp::random(depth - 1)), Box::new(SdfOp::random(depth - 1))),
                2 => {
                    let t = match rng.random_range(0..4) {
                        0 => TransformOp::RotateY(rng.random_range(0.0..6.28)),
                        1 => TransformOp::Translate([rng.random_range(-2.0..2.0), rng.random_range(-2.0..2.0), rng.random_range(-2.0..2.0)]),
                        2 => TransformOp::Scale(rng.random_range(0.8..1.5)),
                        _ => TransformOp::Repeat(rng.random_range(2.0..5.0)),
                    };
                    SdfOp::Transform(Box::new(t), Box::new(SdfOp::random(depth - 1)))
                },
                _ => SdfOp::random(depth - 1),
            }
        }
    }

    pub fn to_wgsl(&self, p_var: &str) -> String {
        match self {
            SdfOp::Box(b) => format!("sdBox({}, vec3<f32>({:.4}, {:.4}, {:.4}))", p_var, b[0], b[1], b[2]),
            SdfOp::Sphere(r) => format!("sdSphere({}, {:.4})", p_var, r),
            SdfOp::Union(a, b) => format!("min({}, {})", a.to_wgsl(p_var), b.to_wgsl(p_var)),
            SdfOp::Subtract(a, b) => format!("max({}, -({}))", a.to_wgsl(p_var), b.to_wgsl(p_var)),
            SdfOp::Transform(t, child) => {
                let new_p = match **t {
                    TransformOp::RotateY(a) => format!("rotY({}, {:.4})", p_var, a),
                    TransformOp::Translate(v) => format!("({} - vec3<f32>({:.4}, {:.4}, {:.4}))", p_var, v[0], v[1], v[2]),
                    TransformOp::Scale(s) => format!("({} / {:.4})", p_var, s),
                    TransformOp::Repeat(c) => format!("(fract({} / {:.4}) * {:.4} - {:.4} * 0.5)", p_var, c, c, c), 
                };
                match **t {
                    TransformOp::Scale(s) => format!("({} * {:.4})", child.to_wgsl(&new_p), s),
                    _ => child.to_wgsl(&new_p),
                }
            }
        }
    }

    pub fn mutate(&self, rate: f32) -> Self {
        let mut rng = rand::rng();
        if rng.random::<f32>() < rate {
            // Structural Mutation: Replace this node entirely
            return SdfOp::random(3); 
        }

        // Parameter Mutation
        match self {
            SdfOp::Box(b) => {
                let mut new_b = *b;
                if rng.random_bool(0.5) {
                    let idx = rng.random_range(0..3);
                    new_b[idx] += (rng.random::<f32>() - 0.5) * 0.2;
                }
                SdfOp::Box(new_b)
            },
            SdfOp::Sphere(r) => {
                let new_r = r + (rng.random::<f32>() - 0.5) * 0.2;
                SdfOp::Sphere(new_r)
            },
            SdfOp::Union(a, b) => SdfOp::Union(Box::new(a.mutate(rate)), Box::new(b.mutate(rate))),
            SdfOp::Subtract(a, b) => SdfOp::Subtract(Box::new(a.mutate(rate)), Box::new(b.mutate(rate))),
            SdfOp::Transform(t, child) => {
                let new_t = match **t {
                    TransformOp::RotateY(a) => TransformOp::RotateY(a + (rng.random::<f32>() - 0.5) * 0.5),
                    TransformOp::Translate(v) => {
                        let mut new_v = v;
                        let idx = rng.random_range(0..3);
                        new_v[idx] += (rng.random::<f32>() - 0.5) * 0.5;
                        TransformOp::Translate(new_v)
                    },
                    TransformOp::Scale(s) => TransformOp::Scale(s + (rng.random::<f32>() - 0.5) * 0.1),
                    TransformOp::Repeat(c) => TransformOp::Repeat(c + (rng.random::<f32>() - 0.5) * 0.5),
                };
                SdfOp::Transform(Box::new(new_t), Box::new(child.mutate(rate)))
            }
        }
    }

    pub fn crossover(parent_a: &Self, parent_b: &Self) -> Self {
        let mut rng = rand::rng();
        // Simple crossover: Swap root logic or pick one parent
        // Real tree crossover swaps subtrees.
        // For now, 50/50 split at root level if compatible types, or just pick one.
        
        match (parent_a, parent_b) {
            (SdfOp::Union(a1, a2), SdfOp::Union(b1, b2)) => {
                SdfOp::Union(Box::new(SdfOp::crossover(a1, b1)), Box::new(SdfOp::crossover(a2, b2)))
            },
            _ => {
                if rng.random_bool(0.5) { parent_a.clone() } else { parent_b.clone() }
            }
        }
    }
}
