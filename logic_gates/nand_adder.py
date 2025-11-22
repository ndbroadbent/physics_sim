import subprocess

def generate_nand_adder_dot(num_bits=8):
    lines = []
    lines.append("digraph NAND_Adder {")
    lines.append("  newrank=true;")  # Key to allow rank=source/sink with clusters
    lines.append("  rankdir=LR;")
    lines.append("  nodesep=0.5;")
    lines.append("  ranksep=0.8;")
    lines.append("  splines=ortho;")
    
    # Define styles
    lines.append("  node [fontname=\"Helvetica\"];")
    lines.append("  edge [fontname=\"Helvetica\", fontsize=10];")
    
    # --- Inputs Grouping ---
    lines.append("  subgraph cluster_inputs {")
    lines.append("    label=\"Inputs\";")
    lines.append("    style=filled; color=\"#f0f0f0\";")
    
    # Cluster A
    lines.append("    subgraph cluster_A {")
    lines.append("      label=\"Input A\";")
    lines.append("      style=filled; color=\"#ffffff\";")
    lines.append("      node [style=filled, fillcolor=\"#e0e0e0\", shape=circle];")
    for i in range(num_bits):
        lines.append(f'      A{i} [label="A{i}"];')
    # Force vertical stacking
    for i in range(num_bits - 1):
        lines.append(f'      A{i} -> A{i+1} [style=invis, weight=100];')
    lines.append("    }")

    # Cluster B
    lines.append("    subgraph cluster_B {")
    lines.append("      label=\"Input B\";")
    lines.append("      style=filled; color=\"#ffffff\";")
    lines.append("      node [style=filled, fillcolor=\"#e0e0e0\", shape=circle];")
    for i in range(num_bits):
        lines.append(f'      B{i} [label="B{i}"];')
    # Force vertical stacking
    for i in range(num_bits - 1):
        lines.append(f'      B{i} -> B{i+1} [style=invis, weight=100];')
    lines.append("    }")
    
    lines.append("    Cin0 [label=\"Cin0=0\", shape=square];")
    lines.append("  }")

    # Force inputs to be at the source rank (leftmost)
    # With newrank=true, we can do this without breaking clusters
    lines.append("  { rank=source; Cin0; " + "; ".join([f"A{i}" for i in range(num_bits)]) + "; " + "; ".join([f"B{i}" for i in range(num_bits)]) + "; }")

    # --- Outputs Grouping ---
    lines.append("  subgraph cluster_S {")
    lines.append("    label=\"Output Sum\";")
    lines.append("    style=filled; color=\"#d0e0ff\";")
    lines.append("    node [shape=doublecircle, style=filled, fillcolor=\"#c0f0c0\"];")
    for i in range(num_bits):
        lines.append(f'    S{i} [label="S{i}"];')
    # Force vertical stacking
    for i in range(num_bits - 1):
        lines.append(f'    S{i} -> S{i+1} [style=invis, weight=100];')
    lines.append("  }")

    # Force outputs to be at the sink rank (rightmost)
    lines.append("  { rank=sink; " + "; ".join([f"S{i}" for i in range(num_bits)]) + "; }")
        
    # --- NAND Nodes Logic ---
    lines.append("  // NAND Gates")
    lines.append("  node [shape=box, style=filled, fillcolor=\"#ffcccc\", label=\"NAND\", height=0.3, width=0.6];")

    # Helper to define a NAND node
    def nand_node(bit, name):
        nid = f"b{bit}_{name}"
        lines.append(f'  {nid};')
        return nid

    prev_cout = "Cin0"
    for bit in range(num_bits):
        # Create a cluster for each bit's adder logic to keep it somewhat organized (optional but nice)
        lines.append(f"  subgraph cluster_bit_{bit} {{")
        lines.append(f"    label=\"Bit {bit}\"; style=dotted; color=\"#aaaaaa\";")
        
        A = f"A{bit}"
        B = f"B{bit}"
        Cin = prev_cout
        
        # Half-adder 1: s1 = A XOR B, c1 = A AND B
        ha1_n1 = nand_node(bit, "ha1_n1")  # nand(A,B)
        lines.append(f"    {A} -> {ha1_n1};")
        lines.append(f"    {B} -> {ha1_n1};")
        
        ha1_n2 = nand_node(bit, "ha1_n2")  # nand(A,ha1_n1)
        lines.append(f"    {A} -> {ha1_n2};")
        lines.append(f"    {ha1_n1} -> {ha1_n2};")
        
        ha1_n3 = nand_node(bit, "ha1_n3")  # nand(B,ha1_n1)
        lines.append(f"    {B} -> {ha1_n3};")
        lines.append(f"    {ha1_n1} -> {ha1_n3};")
        
        s1 = nand_node(bit, "s1")          # nand(ha1_n2,ha1_n3)
        lines.append(f"    {ha1_n2} -> {s1};")
        lines.append(f"    {ha1_n3} -> {s1};")
        
        c1 = nand_node(bit, "c1")          # nand(ha1_n1,ha1_n1) = A AND B
        lines.append(f"    {ha1_n1} -> {c1};")
        lines.append(f"    {ha1_n1} -> {c1};")
        
        # Half-adder 2: S = s1 XOR Cin, c2 = s1 AND Cin
        ha2_n1 = nand_node(bit, "ha2_n1")  # nand(s1, Cin)
        lines.append(f"    {s1} -> {ha2_n1};")
        lines.append(f"    {Cin} -> {ha2_n1};")
        
        ha2_n2 = nand_node(bit, "ha2_n2")  # nand(s1, ha2_n1)
        lines.append(f"    {s1} -> {ha2_n2};")
        lines.append(f"    {ha2_n1} -> {ha2_n2};")
        
        ha2_n3 = nand_node(bit, "ha2_n3")  # nand(Cin, ha2_n1)
        lines.append(f"    {Cin} -> {ha2_n3};")
        lines.append(f"    {ha2_n1} -> {ha2_n3};")
        
        S = nand_node(bit, "S")            # final sum bit S = nand(ha2_n2,ha2_n3)
        lines.append(f"    {ha2_n2} -> {S};")
        lines.append(f"    {ha2_n3} -> {S};")
        
        # connect to external sum output
        lines.append(f"    {S} -> S{bit};")
        
        c2 = nand_node(bit, "c2")          # nand(ha2_n1,ha2_n1) = s1 AND Cin
        lines.append(f"    {ha2_n1} -> {c2};")
        lines.append(f"    {ha2_n1} -> {c2};")
        
        # OR for carry out: Cout = c1 OR c2 via NANDs
        or_n1 = nand_node(bit, "or_n1")    # nand(c1,c1) = NOT c1
        lines.append(f"    {c1} -> {or_n1};")
        lines.append(f"    {c1} -> {or_n1};")
        
        or_n2 = nand_node(bit, "or_n2")    # nand(c2,c2) = NOT c2
        lines.append(f"    {c2} -> {or_n2};")
        lines.append(f"    {c2} -> {or_n2};")
        
        Cout = nand_node(bit, "Cout")      # nand(or_n1, or_n2) = c1 OR c2
        lines.append(f"    {or_n1} -> {Cout};")
        lines.append(f"    {or_n2} -> {Cout};")
        
        lines.append("  }") # End cluster_bit_{bit}

        prev_cout = Cout

    lines.append("}")
    return "\n".join(lines)

if __name__ == "__main__":
    dot_content = generate_nand_adder_dot(8)
    filename = "nand_adder.dot"
    with open(filename, "w") as f:
        f.write(dot_content)
    print(f"Generated {filename}")
    
    svg_filename = "nand_adder.svg"
    try:
        subprocess.run(["dot", "-Tsvg", filename, "-o", svg_filename], check=True)
        print(f"Generated {svg_filename}")
        
        png_filename = "nand_adder.png"
        # Generate a large PNG with 300 DPI
        subprocess.run(["dot", "-Tpng", "-Gdpi=300", filename, "-o", png_filename], check=True)
        print(f"Generated {png_filename} (300 DPI)")
    except FileNotFoundError:
        print("Error: 'dot' command not found. Please install Graphviz.")
    except subprocess.CalledProcessError as e:
        print(f"Error running dot: {e}")
