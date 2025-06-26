use pyo3::prelude::*;

// Declare the data_loader module.
pub mod data_loader;
// Declare the new clustering module.
pub mod clustering;

use pyo3::types::PyDict;
use std::collections::HashMap;

/// Clusters decks from a database and returns a mapping of deck hash to cluster ID.
#[pyfunction]
fn cluster_decks(py: Python, db_path: String, k: usize) -> PyResult<PyObject> {
    // 1. Prepare data for clustering
    let (feature_matrix, deck_hashes) = clustering::prepare_data_for_clustering(&db_path)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to prepare data: {}", e)))?;

    if feature_matrix.is_empty() || deck_hashes.is_empty() {
        // Return an empty dictionary if there's no data to cluster
        return Ok(PyDict::new(py).into());
    }

    // 2. Run K-Means clustering
    let cluster_assignments = clustering::run_kmeans(&feature_matrix, k);

    // 3. Create a HashMap to map deck_hash to cluster_id
    let mut deck_to_cluster: HashMap<String, usize> = HashMap::new();
    for (i, deck_hash) in deck_hashes.iter().enumerate() {
        deck_to_cluster.insert(deck_hash.clone(), cluster_assignments[i]);
    }

    // 4. Convert the Rust HashMap to a Python dictionary
    let py_dict = PyDict::new(py);
    for (key, value) in deck_to_cluster.iter() {
        py_dict.set_item(key, value)?;
    }

    // 5. Return the Python dictionary
    Ok(py_dict.into())
}

/// A Python module implemented in Rust.
#[pymodule]
fn cr_deck_cluster(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(cluster_decks, m)?)?;
    Ok(())
}
