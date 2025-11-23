use serde::{Serialize, Deserialize};
use rand::Rng;

// Better AST for SDF Generation
// An SDF is a function f(p) -> d
// We need nodes that return 'p' (vec3) and nodes that return 'd' (f32).

#[derive(Debug, Clone, PartialEq)]
pub enum SdfOp {
    Sphere(f32),
    Box([f32; 3]),
    Union(Box<SdfOp>, Box<SdfOp>),
    Subtract(Box<SdfOp>, Box<SdfOp>),
    Intersect(Box<SdfOp>, Box<SdfOp>),
    SmoothUnion(Box<SdfOp>, Box<SdfOp>, f32),
    SmoothSubtract(Box<SdfOp>, Box<SdfOp>, f32),
    SmoothIntersect(Box<SdfOp>, Box<SdfOp>, f32),
    Transform(Box<SdfOp>, TransformOp),
    Repeat(f32, Box<SdfOp>),
    Scalar(f32), // New: Represents a constant distance field
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)] // Added PartialEq for derive
pub enum TransformOp {
    RotateY(f32),
    Translate([f32; 3]),
    Scale(f32),
    Repeat(f32),
    Twist(f32), // p.xz = rot(p.y * k) * p.xz
    Fold([f32; 3]), // p = abs(p) - k
}

impl SdfOp {
    pub fn random(depth: u32) -> Self {
        let mut rng = rand::rng();
        // Force structure: Only allow terminals if depth is 0, or very small chance otherwise.
        if depth == 0 || (depth < 3 && rng.random_bool(0.1)) { // Reduced terminal chance
            if rng.random_bool(0.5) {
                SdfOp::Box([rng.random_range(0.5..2.0), rng.random_range(0.5..2.0), rng.random_range(0.5..2.0)])
            } else {
                SdfOp::Sphere(rng.random_range(0.5..2.0))
            }
        } else {
            match rng.random_range(0..6) { // Added Scalar to random ops
                0 => SdfOp::Union(Box::new(SdfOp::random(depth - 1)), Box::new(SdfOp::random(depth - 1))),
                1 => SdfOp::Subtract(Box::new(SdfOp::random(depth - 1)), Box::new(SdfOp::random(depth - 1))),
                2 => SdfOp::SmoothUnion(Box::new(SdfOp::random(depth - 1)), Box::new(SdfOp::random(depth - 1)), rng.random_range(0.1..1.0)),
                3 => SdfOp::Scalar(rng.random_range(-2.0..2.0)), // Random scalar value
                4 | 5 => { // Bias towards transforms (geometry modifiers)
                    let t = match rng.random_range(0..6) {
                        0 => TransformOp::RotateY(rng.random_range(0.0..6.28)),
                        1 => TransformOp::Translate([rng.random_range(-2.0..2.0), rng.random_range(-2.0..2.0), rng.random_range(-2.0..2.0)]),
                        2 => TransformOp::Scale(rng.random_range(0.8..1.5)),
                        3 => TransformOp::Repeat(rng.random_range(2.0..5.0)),
                        4 => TransformOp::Twist(rng.random_range(0.1..1.0)),
                        _ => TransformOp::Fold([rng.random_range(0.1..1.0), rng.random_range(0.1..1.0), rng.random_range(0.1..1.0)]),
                    };
                    SdfOp::Transform(Box::new(SdfOp::random(depth - 1)), t) // Transform needs to be Boxed
                },
                _ => SdfOp::random(depth - 1),
            }
        }
    }

    pub fn to_wgsl(&self, p_var: &str) -> String {
        match self {
            SdfOp::Box(b) => format!("sdBox({}, vec3<f32>({:.4}, {:.4}, {:.4}))", p_var, b[0], b[1], b[2]),
            SdfOp::Sphere(r) => format!("sdSphere({}, {:.4})", p_var, r),
            SdfOp::Union(a, b) => format!("opUnion({}, {})", a.to_wgsl(p_var), b.to_wgsl(p_var)),
            SdfOp::Subtract(a, b) => format!("opSubtract({}, {})", a.to_wgsl(p_var), b.to_wgsl(p_var)),
            SdfOp::Intersect(a, b) => format!("opIntersect({}, {})", a.to_wgsl(p_var), b.to_wgsl(p_var)),
            SdfOp::SmoothUnion(a, b, k) => format!("opSmoothUnion({}, {}, {:.4})", a.to_wgsl(p_var), b.to_wgsl(p_var), *k),
            SdfOp::SmoothSubtract(a, b, k) => format!("opSmoothSubtract({}, {}, {:.4})", a.to_wgsl(p_var), b.to_wgsl(p_var), *k),
            SdfOp::SmoothIntersect(a, b, k) => format!("opSmoothIntersect({}, {}, {:.4})", a.to_wgsl(p_var), b.to_wgsl(p_var), *k),
            SdfOp::Transform(child, t_op) => {
                let (transformed_p_expr, scale_factor_applied_to_distance) = match t_op {
                    TransformOp::RotateY(a) => (format!("rotY({}, {:.4})", p_var, a), 1.0),
                    TransformOp::Translate(v) => (format!("({} - vec3<f32>({:.4}, {:.4}, {:.4}))", p_var, v[0], v[1], v[2]), 1.0),
                    TransformOp::Scale(s) => (format!("({} / {:.4})", p_var, s), *s), // Scale 'p' and apply inverse scale to distance
                    TransformOp::Repeat(c) => (format!("(fract({} / {:.4}) * {:.4} - {:.4} * 0.5)", p_var, c, c, c), 1.0), // Need opRepeat for SDFs
                    TransformOp::Twist(k) => (format!("opTwist({}, {:.4})", p_var, k), 1.0), // Custom opTwist
                    TransformOp::Fold(k) => (format!("opFold({}, vec3<f32>({:.4}, {:.4}, {:.4}))", p_var, k[0], k[1], k[2]), 1.0), // Custom opFold
                };
                let child_sdf = child.to_wgsl(&transformed_p_expr);
                format!("({} * {:.4})", child_sdf, scale_factor_applied_to_distance) 
            },
            SdfOp::Repeat(spacing, op) => format!("opRep({}, {:.4})", op.to_wgsl(p_var), spacing), // opRep uses SDF
            SdfOp::Scalar(val) => format!("{:.4}", val),
        }
    }

    pub fn mutate(&self, rate: f32) -> Self {
        let mut rng = rand::rng();
        if rng.random::<f32>() < rate {
            // Structural Mutation: Replace this node entirely
            return SdfOp::random(3); 
        }

        // Parameter Mutation (Recursive)
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
            SdfOp::Intersect(a, b) => SdfOp::Intersect(Box::new(a.mutate(rate)), Box::new(b.mutate(rate))),
            SdfOp::SmoothUnion(a, b, k) => SdfOp::SmoothUnion(Box::new(a.mutate(rate)), Box::new(b.mutate(rate)), k + (rng.random::<f32>() - 0.5) * 0.1),
            SdfOp::SmoothSubtract(a, b, k) => SdfOp::SmoothSubtract(Box::new(a.mutate(rate)), Box::new(b.mutate(rate)), k + (rng.random::<f32>() - 0.5) * 0.1),
            SdfOp::SmoothIntersect(a, b, k) => SdfOp::SmoothIntersect(Box::new(a.mutate(rate)), Box::new(b.mutate(rate)), k + (rng.random::<f32>() - 0.5) * 0.1),
            SdfOp::Transform(child, t_op) => {
                let new_t_op = match t_op {
                    TransformOp::RotateY(a) => TransformOp::RotateY(a + (rng.random::<f32>() - 0.5) * 0.5),
                    TransformOp::Translate(v) => {
                        let mut new_v = *v;
                        let idx = rng.random_range(0..3);
                        new_v[idx] += (rng.random::<f32>() - 0.5) * 0.5;
                        TransformOp::Translate(new_v)
                    },
                    TransformOp::Scale(s) => TransformOp::Scale(s + (rng.random::<f32>() - 0.5) * 0.1),
                    TransformOp::Repeat(c) => TransformOp::Repeat(c + (rng.random::<f32>() - 0.5) * 0.5),
                    TransformOp::Twist(k) => TransformOp::Twist(k + (rng.random::<f32>() - 0.5) * 0.1),
                    TransformOp::Fold(k) => {
                        let mut new_k = *k;
                        let idx = rng.random_range(0..3);
                        new_k[idx] += (rng.random::<f32>() - 0.5) * 0.1;
                        TransformOp::Fold(new_k)
                    },
                };
                SdfOp::Transform(Box::new(child.mutate(rate)), new_t_op) 
            },
            SdfOp::Repeat(spacing, op) => SdfOp::Repeat(spacing + (rng.random::<f32>() - 0.5) * 0.5, Box::new(op.mutate(rate))),
            SdfOp::Scalar(val) => SdfOp::Scalar(val + (rng.random::<f32>() - 0.5) * 0.1),
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
            (SdfOp::Subtract(a1, a2), SdfOp::Subtract(b1, b2)) => {
                SdfOp::Subtract(Box::new(SdfOp::crossover(a1, b1)), Box::new(SdfOp::crossover(a2, b2)))
            },
            (SdfOp::Transform(child_a, t_op_a), SdfOp::Transform(child_b, t_op_b)) => {
                SdfOp::Transform(Box::new(SdfOp::crossover(child_a, child_b)), if rng.random_bool(0.5) {t_op_a.clone()} else {t_op_b.clone()})
            },
            _ => {
                if rng.random_bool(0.5) { parent_a.clone() } else { parent_b.clone() }
            }
        }
    }
}