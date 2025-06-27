import sqlite3
import pandas as pd
import cr_deck_cluster  # Your Rust module
import os

# --- Configuration ---
DB_PATH = "../cr_data_collector/clash_royale_battles.db"
NUM_CLUSTERS = 20  # Must be the same as used in previous steps

def main():
    """
    Analyzes the card composition of each deck cluster to identify archetypes.
    """
    print("🔬 Cluster Analysis Script Started.")

    # --- Database Check ---
    if not os.path.exists(DB_PATH):
        print(f"❌ Error: Database file not found at '{DB_PATH}'.")
        return
    print(f"✔️ Database found at '{DB_PATH}'")

    # =========================================================================
    # STEP 1: Load Data
    # =========================================================================
    print("\n🚀 STEP 1: Loading data...")

    # Load the cluster mapping from our Rust module
    try:
        print("   - Running Rust clustering function...")
        deck_to_cluster_map = cr_deck_cluster.cluster_decks(DB_PATH, NUM_CLUSTERS)
        if not deck_to_cluster_map:
            print("   - ⚠️ Clustering returned no results. Cannot proceed.")
            return
        print(f"   - ✅ Clustered {len(deck_to_cluster_map)} decks.")
    except Exception as e:
        print(f"   - ❌ An error occurred running the Rust module: {e}")
        return

    # Load necessary tables from the database
    conn = None
    try:
        print("   - Loading tables from SQLite...")
        conn = sqlite3.connect(DB_PATH)
        decks_df = pd.read_sql_query("SELECT * FROM decks", conn)
        card_instances_df = pd.read_sql_query("SELECT * FROM card_instances", conn)
        card_metadata_df = pd.read_sql_query("SELECT * FROM card_metadata", conn)
        print("   - ✅ Tables loaded successfully.")
    except Exception as e:
        print(f"   - ❌ Failed to load data from SQLite: {e}")
        return
    finally:
        if conn:
            conn.close()

    # =========================================================================
    # STEP 2: Combine the Data into a Master DataFrame
    # =========================================================================
    print("\n🚀 STEP 2: Merging data to link clusters to card names...")

    # Convert the map to a DataFrame for merging
    clusters_df = pd.DataFrame(list(deck_to_cluster_map.items()), columns=['deck_hash', 'cluster_id'])

    # Melt the wide decks table to a long format
    card_instance_cols = [f'card_instance_hash_{i}' for i in range(1, 9)]
    decks_long_df = decks_df.melt(
        id_vars=['deck_hash'],
        value_vars=card_instance_cols,
        value_name='instance_hash'
    ).drop(columns=['variable'])

    # Perform the series of merges
    # 1. Merge clusters with decks
    master_df = pd.merge(clusters_df, decks_long_df, on='deck_hash')
    # 2. Merge with card instances
    master_df = pd.merge(master_df, card_instances_df, on='instance_hash')
    # 3. Merge with card metadata to get names
    master_df = pd.merge(master_df, card_metadata_df, left_on='card_type_id', right_on='id')

    print(f"✅ Master DataFrame created with {len(master_df)} total card entries.")


    # =========================================================================
    # STEP 3: Analyze Card Frequency per Cluster
    # =========================================================================
    print("\n🚀 STEP 3: Analyzing card frequency within each cluster...")

    # This creates a multi-index Series: (cluster_id, card_name) -> count
    card_counts = master_df.groupby('cluster_id')['name'].value_counts()
    print("✅ Card frequencies calculated.")


    # =========================================================================
    # STEP 4: Print the Summary
    # =========================================================================
    print("\n" + "="*50)
    print("          Deck Archetype Analysis Results")
    print("="*50 + "\n")

    # Get the unique cluster IDs sorted for consistent output
    unique_clusters = sorted(master_df['cluster_id'].unique())

    for cluster_id in unique_clusters:
        print(f"--- Cluster {cluster_id} ---")
        # Select the data for the current cluster and get the top 8 cards
        top_cards = card_counts.loc[cluster_id].nlargest(8)

        if top_cards.empty:
            print("  No card data available for this cluster.")
        else:
            # Calculate usage percentage within the cluster
            total_cards_in_cluster = master_df[master_df['cluster_id'] == cluster_id].shape[0]
            decks_in_cluster = len(clusters_df[clusters_df['cluster_id'] == cluster_id])

            for card_name, count in top_cards.items():
                # The number of decks a card appears in is its count in this context
                usage_rate = (count / decks_in_cluster) * 100 if decks_in_cluster > 0 else 0
                print(f"  - {card_name:<20} (Used in {usage_rate:.1f}% of decks)")
        print("\n")

    print("🏁 Analysis complete.")


if __name__ == "__main__":
    main()