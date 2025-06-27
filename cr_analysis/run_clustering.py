# File: cr_analysis/run_clustering.py

import cr_deck_cluster
import json
import os

# --- Configuration ---
# Path to the database created by the Rust data collector
# Assumes the script is run from the 'cr_analysis' directory.
DB_PATH = "../cr_data_collector/clash_royale_battles.db"
NUM_CLUSTERS = 20 # Let's start by trying to find 20 archetypes

def main():
    """
    Main function to run the deck clustering and display results.
    """
    print("🐍 Python script started.")

    # --- Database Check ---
    if not os.path.exists(DB_PATH):
        print(f"❌ Error: Database file not found at '{DB_PATH}'.")
        print("Please ensure the data collector has been run and the path is correct.")
        return

    print(f"✔️ Database found at '{DB_PATH}'")

    # --- Rust Module Call ---
    print(f"\n🚀 Calling Rust native function 'cluster_decks' with k={NUM_CLUSTERS}...")
    try:
        # Call the function from our Rust module
        deck_to_cluster_map = cr_deck_cluster.cluster_decks(DB_PATH, NUM_CLUSTERS)
        print(f"✅ Successfully clustered {len(deck_to_cluster_map)} unique decks.")

        # --- Display Results ---
        if deck_to_cluster_map:
            print("\n📊 Sample of results (deck_hash: cluster_id):")
            for i, (deck, cluster) in enumerate(deck_to_cluster_map.items()):
                if i >= 5:
                    break
                print(f"  - Deck {deck[:20]}... assigned to Cluster {cluster}")
        else:
            print("⚠️ The clustering function returned no results. The database might be empty or contain no valid decks.")

    except Exception as e:
        print(f"❌ An error occurred while running the Rust clustering module: {e}")
        print("   Please check the console for any Rust-level panic messages.")


if __name__ == "__main__":
    main()
