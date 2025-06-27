# File: cr_analysis/run_clustering.py

import cr_deck_cluster
import os
import sqlite3
import pandas as pd
import numpy as np
from sklearn.impute import SimpleImputer

# --- Configuration ---
DB_PATH = "../cr_data_collector/clash_royale_battles.db"
NUM_CLUSTERS = 20  # The number of deck archetypes to identify

def main():
    """
    Main function to run the full data processing pipeline with advanced features.
    """
    print("🐍 Python data processing pipeline with ADVANCED feature engineering started.")

    # --- Database Check ---
    if not os.path.exists(DB_PATH):
        print(f"❌ Error: Database file not found at '{DB_PATH}'.")
        return
    print(f"✔️ Database found at '{DB_PATH}'")

    # =========================================================================
    # STEP 0: Run Deck Clustering
    # =========================================================================
    print(f"\n🚀 STEP 0: Calling Rust native function 'cluster_decks'...")
    try:
        deck_to_cluster_map = cr_deck_cluster.cluster_decks(DB_PATH, NUM_CLUSTERS)
        if not deck_to_cluster_map:
            print("⚠️ The clustering function returned no results. Cannot proceed.")
            return
        print(f"✅ Successfully clustered {len(deck_to_cluster_map)} unique decks.")
    except Exception as e:
        print(f"❌ An error occurred while running the Rust clustering module: {e}")
        return

    # =========================================================================
    # NEW STEP 1: Load All Necessary Tables
    # =========================================================================
    print("\n🚀 NEW STEP 1: Loading all necessary tables from SQLite...")
    conn = None
    try:
        conn = sqlite3.connect(DB_PATH)
        battles_df = pd.read_sql_query("SELECT * FROM battles", conn)
        decks_df = pd.read_sql_query("SELECT * FROM decks", conn)
        card_instances_df = pd.read_sql_query("SELECT * FROM card_instances", conn)
        # card_metadata is not strictly needed for this logic but good to have
        # card_metadata_df = pd.read_sql_query("SELECT * FROM card_metadata", conn)
        print(f"✅ Loaded {len(battles_df)} battles, {len(decks_df)} unique decks, and {len(card_instances_df)} unique card instances.")
    except Exception as e:
        print(f"❌ Failed to load data from SQLite: {e}")
        return
    finally:
        if conn:
            conn.close()

    # =========================================================================
    # NEW STEP 2: Create Deck-Level Features
    # =========================================================================
    print("\n🚀 NEW STEP 2: Engineering deck-level features (avg level, evolutions)...")

    # Melt the decks DataFrame from wide to long format
    card_instance_cols = [f'card_instance_hash_{i}' for i in range(1, 9)]
    decks_long_df = decks_df.melt(
        id_vars=['deck_hash'],
        value_vars=card_instance_cols,
        value_name='instance_hash'
    ).drop(columns=['variable']) # Drop the useless 'variable' column

    # Merge with card_instances to get level and evolution details
    decks_with_details_df = pd.merge(decks_long_df, card_instances_df, on='instance_hash')

    # Aggregate stats for each deck
    # Note: For evolution_count, we use 'count'. This counts non-null entries in the
    # 'evolution_level' column, correctly tallying the number of evolved cards.
    deck_features_df = decks_with_details_df.groupby('deck_hash').agg(
        avg_card_level=('level', 'mean'),
        evolution_count=('evolution_level', 'count') # 'count' ignores NaNs, which is what we want.
    )
    print(f"✅ Created features for {len(deck_features_df)} unique decks.")


    # =========================================================================
    # OLD STEPS, now starting from the loaded battles_df
    # =========================================================================
    # Engineer the Target Variable (Player A Wins)
    battles_df = battles_df[battles_df['player_a_crowns'] != battles_df['player_b_crowns']]
    battles_df['player_a_wins'] = (battles_df['player_a_crowns'] > battles_df['player_b_crowns']).astype(int)

    # Add the Synergy Feature (Archetype IDs)
    battles_df['player_a_archetype'] = battles_df['player_a_deck_hash'].map(deck_to_cluster_map)
    battles_df['player_b_archetype'] = battles_df['player_b_deck_hash'].map(deck_to_cluster_map)
    battles_df.dropna(subset=['player_a_archetype', 'player_b_archetype'], inplace=True)
    battles_df['player_a_archetype'] = battles_df['player_a_archetype'].astype(int)
    battles_df['player_b_archetype'] = battles_df['player_b_archetype'].astype(int)

    # =========================================================================
    # NEW STEP 3: Join New Features to Main Battle DataFrame
    # =========================================================================
    print("\n🚀 NEW STEP 3: Joining new deck features into the main battles DataFrame...")

    # Merge for Player A
    battles_df = pd.merge(
        battles_df,
        deck_features_df,
        left_on='player_a_deck_hash',
        right_index=True,
        how='left'
    )
    battles_df.rename(columns={'avg_card_level': 'player_a_avg_card_level', 'evolution_count': 'player_a_evolution_count'}, inplace=True)

    # Merge for Player B
    battles_df = pd.merge(
        battles_df,
        deck_features_df,
        left_on='player_b_deck_hash',
        right_index=True,
        how='left'
    )
    battles_df.rename(columns={'avg_card_level': 'player_b_avg_card_level', 'evolution_count': 'player_b_evolution_count'}, inplace=True)

    print(f"✅ Successfully joined deck features. Final dataset has {len(battles_df)} battles.")

    # =========================================================================
    # NEW STEP 4 & OLD STEPS 4-6: Define Features, Impute, and Save
    # =========================================================================
    print("\n🚀 NEW STEP 4: Defining final feature set with new engineered features...")

    # Update the feature list with our powerful new features
    feature_cols = [
        'player_a_starting_trophies', 'player_b_starting_trophies',
        'player_a_king_tower_hit_points', 'player_b_king_tower_hit_points',
        'player_a_archetype', 'player_b_archetype',
        # --- Newly Added Features ---
        'player_a_avg_card_level', 'player_a_evolution_count',
        'player_b_avg_card_level', 'player_b_evolution_count'
    ]
    target_col = 'player_a_wins'

    # Drop rows where any of the feature columns are null (could happen from merges)
    battles_df.dropna(subset=feature_cols, inplace=True)

    X = battles_df[feature_cols]
    y = battles_df[target_col]
    print(f"✅ Features selected: {feature_cols}")
    print(f"   - Final feature matrix X shape: {X.shape}")
    print(f"   - Final target vector y shape: {y.shape}")

    # Impute missing data (still a good practice, though dropna helps)
    print("\n🚀 Imputing any remaining missing values...")
    imputer = SimpleImputer(strategy='median')
    X_imputed = imputer.fit_transform(X)

    # Save Processed Data
    print("\n🚀 Saving processed data to disk...")
    np.save('X_train.npy', X_imputed)
    np.save('y_train.npy', y.values)
    print("✅ Successfully saved 'X_train.npy' and 'y_train.npy'.")
    print("\n🏁 Advanced data preparation complete!")


if __name__ == "__main__":
    main()