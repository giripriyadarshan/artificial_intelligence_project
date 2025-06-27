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
    Main function to run the full data processing pipeline:
    1. Cluster decks using the Rust module.
    2. Load battle data from SQLite.
    3. Engineer features and the target variable.
    4. Handle missing data.
    5. Save the processed data for model training.
    """
    print("🐍 Python data processing pipeline started.")

    # --- Database Check ---
    if not os.path.exists(DB_PATH):
        print(f"❌ Error: Database file not found at '{DB_PATH}'.")
        print("Please ensure the data collector has been run and the path is correct.")
        return
    print(f"✔️ Database found at '{DB_PATH}'")

    # =========================================================================
    # STEP 0: Run Deck Clustering (from previous step)
    # =========================================================================
    print(f"\n🚀 STEP 0: Calling Rust native function 'cluster_decks' with k={NUM_CLUSTERS}...")
    try:
        deck_to_cluster_map = cr_deck_cluster.cluster_decks(DB_PATH, NUM_CLUSTERS)
        if not deck_to_cluster_map:
            print("⚠️ The clustering function returned no results. Cannot proceed.")
            return
        print(f"✅ Successfully clustered {len(deck_to_cluster_map)} unique decks into {NUM_CLUSTERS} archetypes.")
    except Exception as e:
        print(f"❌ An error occurred while running the Rust clustering module: {e}")
        return

    # =========================================================================
    # STEP 1: Load Battle Data from SQLite
    # =========================================================================
    print("\n🚀 STEP 1: Loading battle data from SQLite into pandas DataFrame...")
    conn = None
    try:
        conn = sqlite3.connect(DB_PATH)
        battles_df = pd.read_sql_query("SELECT * FROM battles", conn)
        print(f"✅ Loaded {len(battles_df)} battle records.")
    except Exception as e:
        print(f"❌ Failed to load data from SQLite: {e}")
        return
    finally:
        if conn:
            conn.close()

    # =========================================================================
    # STEP 2: Engineer the Target Variable (Player A Wins)
    # =========================================================================
    print("\n🚀 STEP 2: Engineering the target variable 'player_a_wins'...")
    # Filter out draws to create a binary classification problem
    battles_df = battles_df[battles_df['player_a_crowns'] != battles_df['player_b_crowns']]
    # Create the target variable: 1 if Player A won, 0 otherwise
    battles_df['player_a_wins'] = (battles_df['player_a_crowns'] > battles_df['player_b_crowns']).astype(int)
    print(f"✅ Filtered out draws. Remaining battles: {len(battles_df)}")
    print(f"   - Player A wins: {battles_df['player_a_wins'].sum()}")
    print(f"   - Player B wins: {len(battles_df) - battles_df['player_a_wins'].sum()}")

    # =========================================================================
    # STEP 3: Add the Synergy Feature (Archetype IDs)
    # =========================================================================
    print("\n🚀 STEP 3: Mapping deck hashes to cluster IDs (archetypes)...")
    battles_df['player_a_archetype'] = battles_df['player_a_deck_hash'].map(deck_to_cluster_map)
    battles_df['player_b_archetype'] = battles_df['player_b_deck_hash'].map(deck_to_cluster_map)

    # Proactive Step: Handle battles where one or both decks weren't in the cluster map
    initial_rows = len(battles_df)
    battles_df.dropna(subset=['player_a_archetype', 'player_b_archetype'], inplace=True)
    # Convert archetype IDs to integers, as they are now guaranteed to be present
    battles_df['player_a_archetype'] = battles_df['player_a_archetype'].astype(int)
    battles_df['player_b_archetype'] = battles_df['player_b_archetype'].astype(int)
    rows_dropped = initial_rows - len(battles_df)
    print(f"✅ Mapped archetypes. Dropped {rows_dropped} battles with unknown deck archetypes.")
    print(f"   Remaining battles for training: {len(battles_df)}")

    # =========================================================================
    # STEP 4: Define Features (X) and Target (y)
    # =========================================================================
    print("\n🚀 STEP 4: Defining feature matrix (X) and target vector (y)...")
    feature_cols = [
        'player_a_starting_trophies', 'player_b_starting_trophies',
        'player_a_king_tower_hit_points', 'player_b_king_tower_hit_points',
        'player_a_archetype', 'player_b_archetype'
    ]
    target_col = 'player_a_wins'

    X = battles_df[feature_cols]
    y = battles_df[target_col]
    print(f"✅ Features selected: {feature_cols}")
    print(f"   - Feature matrix X shape: {X.shape}")
    print(f"   - Target vector y shape: {y.shape}")

    # =========================================================================
    # STEP 5: Handle Missing Data
    # =========================================================================
    print("\n🚀 STEP 5: Imputing missing values using median strategy...")
    # The trophy and hitpoint columns can be NULL from the API
    imputer = SimpleImputer(strategy='median')
    X_imputed = imputer.fit_transform(X)
    print("✅ Missing values handled.")
    print(f"   - Shape of final feature matrix: {X_imputed.shape}")
    # Note: Scikit-learn's imputer returns a NumPy array, not a DataFrame

    # =========================================================================
    # STEP 6: Save Processed Data
    # =========================================================================
    print("\n🚀 STEP 6: Saving processed data to disk...")
    np.save('X_train.npy', X_imputed)
    np.save('y_train.npy', y.values) # .values ensures we save the underlying numpy array
    print("✅ Successfully saved 'X_train.npy' and 'y_train.npy'.")
    print("\n🏁 Data preparation complete!")


if __name__ == "__main__":
    main()
