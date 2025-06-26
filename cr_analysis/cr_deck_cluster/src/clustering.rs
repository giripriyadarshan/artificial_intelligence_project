use crate::data_loader;
use ndarray::{Array1, Array2, Axis};
use rusqlite::{Connection, Error};
use std::collections::HashMap;

/// Prepares deck data for clustering by converting it into a multi-hot encoded matrix.
///
/// # Arguments
/// * `db_path` - The file path to the SQLite database.
///
/// # Returns
/// A `Result` containing a tuple on success:
/// - `ndarray::Array2<f64>`: A 2D matrix where each row is a deck and each
///   column represents a unique card. A value of 1.0 indicates the presence
///   of a card in the deck.
/// - `Vec<String>`: The list of deck hashes, in the same order as the matrix rows.
pub fn prepare_data_for_clustering(db_path: &str) -> Result<(Array2<f64>, Vec<String>), Error> {
    // a. Load Raw Deck Data
    // This gives us a vector of (deck_hash, Vec<card_type_id>) tuples.
    let loaded_decks = data_loader::load_unique_decks(db_path)?;
    let (deck_hashes, deck_cards): (Vec<_>, Vec<_>) = loaded_decks.into_iter().unzip();

    // b. Create Card Vocabulary
    // This map will associate each unique card ID with a unique column index (0..N-1).
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT id FROM card_metadata ORDER BY id ASC")?;
    let card_id_iter = stmt.query_map([], |row| row.get::<_, u32>(0))?;

    let mut card_vocabulary: HashMap<u32, usize> = HashMap::new();
    for (index, card_id_result) in card_id_iter.enumerate() {
        let card_id = card_id_result?;
        card_vocabulary.insert(card_id, index);
    }
    let vocab_size = card_vocabulary.len();
    if vocab_size == 0 {
        // Handle case where database has no cards.
        return Ok((Array2::zeros((0, 0)), Vec::new()));
    }

    // c. Multi-Hot Encode each deck
    let mut encoded_vectors: Vec<Array1<f64>> = Vec::with_capacity(deck_cards.len());
    for card_list in deck_cards {
        // i. Create a vector of zeros with a length equal to the number of unique cards.
        let mut encoded_vector = Array1::zeros(vocab_size);
        // ii. For each card in the deck, set the corresponding index to 1.0.
        for card_id in card_list {
            if let Some(index) = card_vocabulary.get(&card_id) {
                encoded_vector[*index] = 1.0;
            }
        }
        encoded_vectors.push(encoded_vector);
    }

    // d. Stack the 1D vectors into a 2D matrix
    // We need to create views of our 1D arrays to stack them.
    let views: Vec<_> = encoded_vectors.iter().map(|a| a.view()).collect();
    let data_matrix = ndarray::stack(Axis(0), &views)
        .expect("Failed to stack arrays; this should not happen if all vectors have the same length.");

    // e. Return the matrix and the corresponding deck hashes
    Ok((data_matrix, deck_hashes))
}

/// Performs K-Means clustering on the given dataset.
///
/// # Arguments
/// * `data` - A 2D array representing the dataset, where rows are samples and columns are features.
/// * `k` - The number of clusters to form.
///
/// # Returns
/// A 1D array containing the cluster assignments for each data point.
pub fn run_kmeans(data: &Array2<f64>, k: usize) -> Array1<usize> {
    use linfa_clustering::KMeans;
    use linfa::prelude::*;
    use rand_xoshiro::Xoshiro256Plus; // Use Xoshiro256Plus
    use rand::SeedableRng;
    use linfa::DatasetBase; // Import DatasetBase

    let dataset = DatasetBase::from(data.clone()); // Convert data to DatasetBase

    let rng = Xoshiro256Plus::seed_from_u64(42); // Seed Xoshiro256Plus
    let model = KMeans::params_with_rng(k, rng)
        .fit(&dataset) // Pass DatasetBase to fit
        .expect("KMeans fitting failed");

    model.predict(&dataset) // Pass DatasetBase to predict
}
