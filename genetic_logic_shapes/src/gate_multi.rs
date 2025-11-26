use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operation {
    AND, OR, XOR, NAND, NOR, NOT, WIRE,
}

impl Operation {
    pub fn random(rng: &mut impl Rng) -> Self {
        match rng.gen_range(0..7) {
            0 => Operation::AND,
            1 => Operation::OR,
            2 => Operation::XOR,
            3 => Operation::NAND,
            4 => Operation::NOR,
            5 => Operation::NOT,
            _ => Operation::WIRE,
        }
    }

    pub fn eval(&self, a: bool, b: bool) -> bool {
        match self {
            Operation::AND => a & b,
            Operation::OR => a | b,
            Operation::XOR => a ^ b,
            Operation::NAND => !(a & b),
            Operation::NOR => !(a | b),
            Operation::NOT => !a,
            Operation::WIRE => a,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub op: Operation,
    pub in_a: usize,
    pub in_b: usize,
}

#[derive(Debug, Clone)]
pub struct Genome {
    pub num_inputs: usize,
    pub nodes: Vec<Node>,
    pub outputs: Vec<usize>, // Multiple outputs
}

impl Genome {
    pub fn new_random(num_inputs: usize, num_nodes: usize, num_outputs: usize, rng: &mut impl Rng) -> Self {
        let mut nodes = Vec::with_capacity(num_nodes);
        for i in 0..num_nodes {
            let limit = num_inputs + i;
            nodes.push(Node {
                op: Operation::random(rng),
                in_a: rng.gen_range(0..limit.max(1)),
                in_b: rng.gen_range(0..limit.max(1)),
            });
        }
        
        let mut outputs = Vec::with_capacity(num_outputs);
        let limit = num_inputs + num_nodes;
        for _ in 0..num_outputs {
            outputs.push(rng.gen_range(0..limit));
        }

        Genome { num_inputs, nodes, outputs }
    }

    pub fn mutate(&mut self, mutation_rate: f64, rng: &mut impl Rng) {
        // Mutate Nodes
        for (i, node) in self.nodes.iter_mut().enumerate() {
            if rng.gen_bool(mutation_rate) {
                match rng.gen_range(0..3) {
                    0 => node.op = Operation::random(rng),
                    1 => {
                        let limit = self.num_inputs + i;
                        node.in_a = rng.gen_range(0..limit.max(1));
                    },
                    2 => {
                        let limit = self.num_inputs + i;
                        node.in_b = rng.gen_range(0..limit.max(1));
                    },
                    _ => {}
                }
            }
        }
        
        // Mutate Outputs
        for out in &mut self.outputs {
            if rng.gen_bool(mutation_rate) {
                let limit = self.num_inputs + self.nodes.len();
                *out = rng.gen_range(0..limit);
            }
        }
    }

    pub fn eval(&self, inputs: &[bool]) -> Vec<bool> {
        let mut values = Vec::with_capacity(self.num_inputs + self.nodes.len());
        values.extend_from_slice(inputs);

        for node in &self.nodes {
            let val_a = values.get(node.in_a).copied().unwrap_or(false);
            let val_b = values.get(node.in_b).copied().unwrap_or(false);
            values.push(node.op.eval(val_a, val_b));
        }

        self.outputs.iter().map(|&idx| values.get(idx).copied().unwrap_or(false)).collect()
    }
}
