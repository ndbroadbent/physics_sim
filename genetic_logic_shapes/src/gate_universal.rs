use rand::Rng;

// A Universal Node uses a 4-bit truth table (u8) to represent ANY binary function.
#[derive(Debug, Clone, Copy)]
pub struct UniversalNode {
    pub table: u8, // Only low 4 bits used
    pub in_a: u16,
    pub in_b: u16,
}

#[derive(Debug, Clone)]
pub struct Genome {
    pub num_inputs: usize,
    pub nodes: Vec<UniversalNode>,
    pub output_node: usize,
}

impl Genome {
    pub fn new_random(num_inputs: usize, num_nodes: usize, rng: &mut impl Rng) -> Self {
        let mut nodes = Vec::with_capacity(num_nodes);
        for i in 0..num_nodes {
            let limit = num_inputs + i;
            nodes.push(UniversalNode {
                table: rng.gen_range(0..16) as u8, // 4-bit truth table
                in_a: rng.gen_range(0..limit) as u16,
                in_b: rng.gen_range(0..limit) as u16,
            });
        }
        
        Genome {
            num_inputs,
            nodes,
            output_node: rng.gen_range(0..num_inputs + num_nodes),
        }
    }

    pub fn mutate(&mut self, mutation_rate: f64, rng: &mut impl Rng) {
        for (i, node) in self.nodes.iter_mut().enumerate() {
            if rng.gen_bool(mutation_rate) {
                let mutation_type = rng.gen_range(0..3);
                match mutation_type {
                    0 => node.table = rng.gen_range(0..16) as u8, // Change function
                    1 => {
                        let limit = self.num_inputs + i;
                        node.in_a = rng.gen_range(0..limit) as u16;
                    },
                    2 => {
                        let limit = self.num_inputs + i;
                        node.in_b = rng.gen_range(0..limit) as u16;
                    },
                    _ => {}
                }
            }
        }
        
        // Mutate output pointer
        if rng.gen_bool(mutation_rate) {
            let limit = self.num_inputs + self.nodes.len();
            self.output_node = rng.gen_range(0..limit);
        }
    }

    pub fn eval(&self, inputs: &[bool]) -> bool {
        let mut values = Vec::with_capacity(self.num_inputs + self.nodes.len());
        values.extend_from_slice(inputs);

        for node in &self.nodes {
            let a = *values.get(node.in_a as usize).unwrap_or(&false);
            let b = *values.get(node.in_b as usize).unwrap_or(&false);
            
            // Truth Table Lookup
            // Index = (A << 1) | B
            let idx = ((a as u8) << 1) | (b as u8);
            let res = (node.table >> idx) & 1;
            
            values.push(res == 1);
        }

        *values.get(self.output_node).unwrap_or(&false)
    }

    pub fn get_active_genome(&self) -> Genome {
        let mut active = vec![false; self.nodes.len()];
        let mut stack = Vec::new();
        
        // Start from output node
        if self.output_node >= self.num_inputs {
            let node_idx = self.output_node - self.num_inputs;
            if node_idx < self.nodes.len() {
                stack.push(node_idx);
                active[node_idx] = true;
            }
        }

        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx];
            
            // Check Input A
            if node.in_a as usize >= self.num_inputs {
                let input_node_idx = node.in_a as usize - self.num_inputs;
                if input_node_idx < self.nodes.len() && !active[input_node_idx] {
                    active[input_node_idx] = true;
                    stack.push(input_node_idx);
                }
            }

            // Check Input B
            if node.in_b as usize >= self.num_inputs {
                let input_node_idx = node.in_b as usize - self.num_inputs;
                if input_node_idx < self.nodes.len() && !active[input_node_idx] {
                    active[input_node_idx] = true;
                    stack.push(input_node_idx);
                }
            }
        }

        // Create mapping
        let mut new_nodes = Vec::new();
        let mut index_map = vec![0u16; self.nodes.len()];
        
        for (old_idx, &is_active) in active.iter().enumerate() {
            if is_active {
                index_map[old_idx] = new_nodes.len() as u16;
                new_nodes.push(self.nodes[old_idx]);
            }
        }

        // Remap inputs
        for node in &mut new_nodes {
            if node.in_a as usize >= self.num_inputs {
                let old_idx = node.in_a as usize - self.num_inputs;
                node.in_a = (self.num_inputs as u16) + index_map[old_idx];
            }
            if node.in_b as usize >= self.num_inputs {
                let old_idx = node.in_b as usize - self.num_inputs;
                node.in_b = (self.num_inputs as u16) + index_map[old_idx];
            }
        }

        // Remap output
        let new_output = if self.output_node >= self.num_inputs {
            let old_idx = self.output_node - self.num_inputs;
            self.num_inputs + index_map[old_idx] as usize
        } else {
            self.output_node
        };

        Genome {
            num_inputs: self.num_inputs,
            nodes: new_nodes,
            output_node: new_output,
        }
    }

    // Serialize the genome structure into a bit vector
    pub fn to_bits(&self) -> Vec<bool> {
        let mut bits = Vec::new();
        let ptr_bits = 12; 

        for node in &self.nodes {
            // Table (4 bits)
            for i in 0..4 { bits.push((node.table >> i) & 1 == 1); }
            // In A (12 bits)
            for i in 0..ptr_bits { bits.push((node.in_a >> i) & 1 == 1); }
            // In B (12 bits)
            for i in 0..ptr_bits { bits.push((node.in_b >> i) & 1 == 1); }
        }
        
        // Output Node (12 bits)
        for i in 0..ptr_bits { bits.push((self.output_node as u16 >> i) & 1 == 1); } // cast to u16
        
        bits
    }
}
