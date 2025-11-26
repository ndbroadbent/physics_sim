use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operation {
    // Basic Binary
    AND, OR, XOR, NAND, NOR,
    // Basic Unary (using only input A)
    NOT,
    // Pass-through (identity)
    WIRE, 
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

    // Bit-parallel evaluation on u64 (simulating 64 pixels at once)
    #[inline(always)]
    pub fn eval(&self, a: u64, b: u64) -> u64 {
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
    pub in_a: usize, // Index in the 'values' array (inputs + previous nodes)
    pub in_b: usize,
}

#[derive(Debug, Clone)]
pub struct Genome {
    pub num_inputs: usize,
    pub nodes: Vec<Node>,
    pub output_node_idx: usize,
}

impl Genome {
    pub fn new_random(num_inputs: usize, num_nodes: usize, rng: &mut impl Rng) -> Self {
        let mut nodes = Vec::with_capacity(num_nodes);

        for i in 0..num_nodes {
            // Inputs to a node can be any previous node OR system input
            // The index space is: [0..num_inputs] are system inputs.
            // [num_inputs..num_inputs+i] are previous nodes.
            // This ensures DAG (Feed-Forward) property, preventing cycles for now to simplify evaluation.
            let limit = num_inputs + i; 
            
            nodes.push(Node {
                op: Operation::random(rng),
                in_a: rng.gen_range(0..limit),
                in_b: rng.gen_range(0..limit),
            });
        }

        Genome {
            num_inputs,
            nodes,
            output_node_idx: rng.gen_range(0..num_inputs + num_nodes),
        }
    }

    pub fn mutate(&mut self, mutation_rate: f64, rng: &mut impl Rng) {
        // Mutate Output Node
        if rng.gen_bool(mutation_rate) {
             let limit = self.num_inputs + self.nodes.len();
             self.output_node_idx = rng.gen_range(0..limit);
        }

        // Mutate Nodes
        for (i, node) in self.nodes.iter_mut().enumerate() {
            if rng.gen_bool(mutation_rate) {
                // Pick which part of the node to mutate
                match rng.gen_range(0..3) {
                    0 => node.op = Operation::random(rng),
                    1 => {
                        let limit = self.num_inputs + i;
                        node.in_a = rng.gen_range(0..limit);
                    },
                    2 => {
                        let limit = self.num_inputs + i;
                        node.in_b = rng.gen_range(0..limit);
                    },
                    _ => {}
                }
            }
        }
    }

    // Evaluates the network. 
    // `inputs` must be exactly `num_inputs` long. 
    // Returns the result of the output node.
    // We reuse a 'buffer' to avoid allocations if provided.
    pub fn eval(&self, inputs: &[u64], buffer: &mut Vec<u64>) -> u64 {
        buffer.clear();
        buffer.extend_from_slice(inputs);

        for node in &self.nodes {
            let val_a = buffer[node.in_a];
            let val_b = buffer[node.in_b];
            let res = node.op.eval(val_a, val_b);
            buffer.push(res);
        }

        buffer[self.output_node_idx]
    }
    
    pub fn active_node_count(&self) -> usize {
        let mut active = vec![false; self.nodes.len()];
        let mut stack = Vec::new();
        
        // Start from output node
        if self.output_node_idx >= self.num_inputs {
            let node_idx = self.output_node_idx - self.num_inputs;
            if node_idx < self.nodes.len() {
                stack.push(node_idx);
                active[node_idx] = true;
            }
        }

        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx];
            
            // Check Input A
            if node.in_a >= self.num_inputs {
                let input_node_idx = node.in_a - self.num_inputs;
                if input_node_idx < self.nodes.len() && !active[input_node_idx] {
                    active[input_node_idx] = true;
                    stack.push(input_node_idx);
                }
            }

            // Check Input B
            if node.in_b >= self.num_inputs {
                let input_node_idx = node.in_b - self.num_inputs;
                if input_node_idx < self.nodes.len() && !active[input_node_idx] {
                    active[input_node_idx] = true;
                    stack.push(input_node_idx);
                }
            }
        }

        active.iter().filter(|&&x| x).count()
    }

    pub fn compact(&self) -> Self {
        let mut active = vec![false; self.nodes.len()];
        let mut stack = Vec::new();
        
        // Start from output node
        if self.output_node_idx >= self.num_inputs {
            let node_idx = self.output_node_idx - self.num_inputs;
            if node_idx < self.nodes.len() {
                stack.push(node_idx);
                active[node_idx] = true;
            }
        }

        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx];
            
            // Check Input A
            if node.in_a >= self.num_inputs {
                let input_node_idx = node.in_a - self.num_inputs;
                if input_node_idx < self.nodes.len() && !active[input_node_idx] {
                    active[input_node_idx] = true;
                    stack.push(input_node_idx);
                }
            }

            // Check Input B
            if node.in_b >= self.num_inputs {
                let input_node_idx = node.in_b - self.num_inputs;
                if input_node_idx < self.nodes.len() && !active[input_node_idx] {
                    active[input_node_idx] = true;
                    stack.push(input_node_idx);
                }
            }
        }

        // Create mapping from old index -> new index
        let mut new_nodes = Vec::new();
        let mut index_map = vec![0usize; self.nodes.len()]; // maps old_node_idx -> new_node_idx
        
        for (old_idx, &is_active) in active.iter().enumerate() {
            if is_active {
                index_map[old_idx] = new_nodes.len();
                new_nodes.push(self.nodes[old_idx].clone());
            }
        }

        // Remap inputs of new nodes
        for node in &mut new_nodes {
            if node.in_a >= self.num_inputs {
                let old_idx = node.in_a - self.num_inputs;
                // If the input node was active (it must be!), remap it
                node.in_a = self.num_inputs + index_map[old_idx];
            }
            if node.in_b >= self.num_inputs {
                let old_idx = node.in_b - self.num_inputs;
                node.in_b = self.num_inputs + index_map[old_idx];
            }
        }

        // Remap output node
        let new_output_idx = if self.output_node_idx >= self.num_inputs {
            let old_idx = self.output_node_idx - self.num_inputs;
            self.num_inputs + index_map[old_idx]
        } else {
            self.output_node_idx
        };

        Genome {
            num_inputs: self.num_inputs,
            nodes: new_nodes,
            output_node_idx: new_output_idx,
        }
    }

    pub fn merge(&self, other: &Genome) -> Self {
        let mut new_nodes = self.nodes.clone();
        let offset = self.nodes.len();
        
        // Append other's nodes with remapped indices
        for node in &other.nodes {
            let mut new_node = node.clone();
            
            if new_node.in_a >= self.num_inputs {
                new_node.in_a += offset;
            }
            if new_node.in_b >= self.num_inputs {
                new_node.in_b += offset;
            }
            
            new_nodes.push(new_node);
        }
        
        // Use other's output node (remapped)
        let new_output_idx = if other.output_node_idx >= self.num_inputs {
            other.output_node_idx + offset
        } else {
            other.output_node_idx
        };

        Genome {
            num_inputs: self.num_inputs,
            nodes: new_nodes,
            output_node_idx: new_output_idx,
        }
    }
}
