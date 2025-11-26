import os
import pandas as pd
import matplotlib.pyplot as plt
import glob

def plot_threshold_sweep(modulus=17):
    csv_files = glob.glob(f'frames/telemetry_m{modulus}_s*.csv')
    csv_files.sort(key=lambda x: int(x.split('_s')[1].split('.')[0])) # Sort by threshold

    plt.figure(figsize=(12, 8))
    
    for file in csv_files:
        try:
            df = pd.read_csv(file)
            if df.empty: continue
            threshold = df['Threshold'].iloc[0]
            
            plt.plot(df['Frame'], df['TotalActive'], label=f'Threshold {threshold} ({threshold}/{modulus})', alpha=0.7)
        except Exception as e:
            print(f"Error processing {file}: {e}")
            
    plt.xlabel('Frame')
    plt.ylabel('Total Active Cells')
    plt.title(f'Activity Curves for Modulus {modulus} (Threshold Sweep)')
    plt.legend()
    plt.grid(True, which="both", ls="--", alpha=0.5)
    plt.yscale('log')
    
    output_file = f'charts/activity_m{modulus}_log.png'
    plt.savefig(output_file)
    print(f"Saved {output_file}")

if __name__ == "__main__":
    if not os.path.exists('charts'):
        os.makedirs('charts')
    # plot_survival_vs_modulus() # Disable old plots
    # plot_activity_curves()
    plot_threshold_sweep(17)
    plot_threshold_sweep(170)
