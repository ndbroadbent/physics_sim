import subprocess

def generate_nand_adder_dot(num_bits=8):
    lines = []
    lines.append("digraph NAND_Adder {")
    # lines.append("  newrank=true;") # Removed to prevent crash
    lines.append("  rankdir=LR;")
    lines.append("  nodesep=0.5;")
    lines.append("  ranksep=1.2;") 
    lines.append("  splines=ortho;")
    lines.append("  bgcolor=\"#000000\";")
    lines.append("  compound=true;") # Allow edges between clusters
    
    # Dark mode styles
    lines.append("  node [fontname=\"Helvetica\", fontcolor=\"#ffffff\"];")
    lines.append("  edge [fontname=\"Helvetica\", fontsize=10, color=\"#ffffff\", fontcolor=\"#ffffff\"];")
    
    def create_node(nid):
        lines.append(f'    {nid};')
        return nid

    # --- CLUSTER 0: INPUTS ---
    lines.append("  subgraph cluster_0_inputs {")
    lines.append("    label=\"Inputs\"; fontcolor=\"#ffffff\";")
    lines.append("    style=filled; color=\"#222222\"; bgcolor=\"#111111\";")
    
    # Define nodes first
    lines.append("    Cin0 [label=\"Cin0=0\", shape=square, fontcolor=\"#ffffff\", fillcolor=\"#444444\"];")
    
    for i in range(num_bits):
        lines.append(f'    A{i} [label="A{i}", style=filled, fillcolor=\"#444444\", shape=circle, fontcolor=\"#ffffff\"];')
        lines.append(f'    B{i} [label="B{i}", style=filled, fillcolor=\"#444444\", shape=circle, fontcolor=\"#ffffff\"];')

    # Vertical alignment edges for Inputs
    lines.append("    edge [style=invis];")
    lines.append("    Cin0 -> A0;") # Cin at top or near A0
    for i in range(num_bits - 1):
        lines.append(f"    A{i} -> A{i+1};")
        lines.append(f"    B{i} -> B{i+1};")
    lines.append("    edge [style=solid];")
    lines.append("  }") 

    # --- CLUSTER 1: ADDERS ---
    lines.append("  subgraph cluster_1_adders {")
    lines.append("    label=\"Full Adders Stack\"; fontcolor=\"#ffffff\";")
    lines.append("    style=filled; color=\"#444444\"; bgcolor=\"#222222\";")
    lines.append("    node [shape=box, style=filled, fillcolor=\"#662222\", label=\"NAND\", height=0.3, width=0.6, fontcolor=\"#ffffff\"];")

    prev_cout = "Cin0"
    for bit in range(num_bits):
        # Nested cluster for each bit to group its logic visually
        lines.append(f"    subgraph cluster_bit_{bit} {{")
        lines.append(f"      label=\"Bit {bit}\"; style=rounded; color=\"#bbbbbb\"; bgcolor=\"#333322\"; fontcolor=\"#ffffff\";")
        
        A = f"A{bit}"
        B = f"B{bit}"
        Cin = prev_cout
        
        # Logic Nodes
        ha1_n1 = create_node(f"b{bit}_ha1_n1")
        c1 = create_node(f"b{bit}_c1")
        ha1_n2 = create_node(f"b{bit}_ha1_n2")
        ha1_n3 = create_node(f"b{bit}_ha1_n3")
        s1 = create_node(f"b{bit}_s1")
        
        ha2_n1 = create_node(f"b{bit}_ha2_n1")
        c2 = create_node(f"b{bit}_c2")
        ha2_n2 = create_node(f"b{bit}_ha2_n2")
        ha2_n3 = create_node(f"b{bit}_ha2_n3")
        S_local = create_node(f"b{bit}_S")
        
        or_n1 = create_node(f"b{bit}_or_n1")
        or_n2 = create_node(f"b{bit}_or_n2")
        Cout = create_node(f"b{bit}_Cout")

        # Edges
        # HA1
        lines.append(f"      {A} -> {ha1_n1}; {B} -> {ha1_n1};")
        lines.append(f"      {ha1_n1} -> {c1}; {ha1_n1} -> {c1};") 
        lines.append(f"      {A} -> {ha1_n2}; {ha1_n1} -> {ha1_n2};")
        lines.append(f"      {B} -> {ha1_n3}; {ha1_n1} -> {ha1_n3};")
        lines.append(f"      {ha1_n2} -> {s1}; {ha1_n3} -> {s1};")

        # HA2
        cin_attr = " [constraint=false, color=\"#00bfff\"]" if bit > 0 else ""
        lines.append(f"      {s1} -> {ha2_n1}; {Cin} -> {ha2_n1}{cin_attr};")
        lines.append(f"      {ha2_n1} -> {c2}; {ha2_n1} -> {c2};")
        lines.append(f"      {s1} -> {ha2_n2}; {ha2_n1} -> {ha2_n2};")
        lines.append(f"      {Cin} -> {ha2_n3}{cin_attr}; {ha2_n1} -> {ha2_n3};")
        lines.append(f"      {ha2_n2} -> {S_local}; {ha2_n3} -> {S_local};")

        # OR
        lines.append(f"      {c1} -> {or_n1}; {c1} -> {or_n1};")
        lines.append(f"      {c2} -> {or_n2}; {c2} -> {or_n2};")
        lines.append(f"      {or_n1} -> {Cout}; {or_n2} -> {Cout};")

        lines.append("    }") # End bit cluster

        prev_cout = Cout

    # Vertical alignment backbone for Adders
    lines.append("    edge [style=invis];")
    for bit in range(num_bits - 1):
        # Link the Couts to force the blocks to stack
        lines.append(f"    b{bit}_Cout -> b{bit+1}_Cout;")
        # Maybe link logic too?
        lines.append(f"    b{bit}_ha1_n1 -> b{bit+1}_ha1_n1;")
    lines.append("    edge [style=solid];")

    lines.append("  }") # End cluster_1_adders

    # --- CLUSTER 2: OUTPUTS ---
    lines.append("  subgraph cluster_2_outputs {")
    lines.append("    label=\"Output Sum\"; fontcolor=\"#ffffff\";")
    lines.append("    style=filled; color=\"#222266\"; bgcolor=\"#111133\";")
    lines.append("    node [shape=doublecircle, style=filled, fillcolor=\"#226622\", fontcolor=\"#ffffff\"];")
    
    for i in range(num_bits):
        lines.append(f'    S{i} [label="S{i}"];')
        
    # Vertical alignment for Outputs
    lines.append("    edge [style=invis];")
    for i in range(num_bits - 1):
        lines.append(f"    S{i} -> S{i+1};")
    lines.append("    edge [style=solid];")
    lines.append("  }") # End cluster_2_outputs

    # --- GLOBAL EDGES (Crossing Clusters) ---
    for bit in range(num_bits):
        lines.append(f"  b{bit}_S -> S{bit};")

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
