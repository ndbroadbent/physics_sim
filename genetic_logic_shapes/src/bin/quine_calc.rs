fn main() {
    println!("Quine Threshold Calculator");
    println!("--------------------------");
    println!("Nodes | PtrBits | Bits/Node | Total Bits | Min Inputs (K) | Capacity (2^K)");
    println!("--------------------------------------------------------------------------");

    for n in 1..=1000 {
        // Inputs constant (e.g. 11 in our experiment).
        // But PtrBits depends on Total Addressable Nodes (Inputs + Nodes).
        // Let's assume Inputs is small (e.g. 11 or ceil(log2(TotalBits))).
        // This is recursive.
        
        // Let's assume Input Bits K is sufficient to address the result.
        // PtrBits = ceil(log2(K + N)).
        
        // Iterative solver for K and PtrBits
        let mut k = 1;
        let mut total_bits = 0;
        
        loop {
            let addressable_space = 1 << k;
            let ptr_bits = ((k + n) as f64).log2().ceil() as usize;
            
            // Genome Bits:
            // N * (4 + 2 * ptr_bits) + ptr_bits (output node)
            let genome_bits = n * (4 + 2 * ptr_bits) + ptr_bits;
            
            if addressable_space >= genome_bits {
                total_bits = genome_bits;
                break;
            }
            k += 1;
        }
        
        // Calculate PtrBits for final display
        let ptr_bits = ((k + n) as f64).log2().ceil() as usize;
        let bits_per_node = 4 + 2 * ptr_bits;
        let capacity = 1 << k;

        println!("{:5} | {:7} | {:9} | {:10} | {:14} | {:14}", 
            n, ptr_bits, bits_per_node, total_bits, k, capacity);
            
        if n % 50 == 0 {
             println!("--------------------------------------------------------------------------");
        }
    }
}
