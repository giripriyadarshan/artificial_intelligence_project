use rusqlite::{Connection, Result};
use std::collections::HashMap;

/// Loads all unique decks from the database and resolves their card instance hashes
/// into card type IDs.
///
/// # Arguments
/// * `db_path` - The file path to the SQLite database.
///
/// # Returns
/// A `Result` containing a vector of tuples on success. Each tuple consists of:
/// - `String`: The unique hash of a deck.
/// - `Vec<u32>`: A vector of the 8 card type IDs that make up the deck.
pub fn load_unique_decks(db_path: &str) -> Result<Vec<(String, Vec<u32>)>> {
    // 1. Open a connection to the database.
    let conn = Connection::open(db_path)?;

    // 2. Create a lookup map from instance_hash to card_type_id.
    let mut instance_to_card_id_map: HashMap<String, u32> = HashMap::new();
    {
        let mut stmt = conn.prepare("SELECT instance_hash, card_type_id FROM card_instances")?;
        let instance_iter = stmt.query_map([], |row| {
            let instance_hash: String = row.get(0)?;
            let card_type_id: u32 = row.get(1)?;
            Ok((instance_hash, card_type_id))
        })?;

        for instance in instance_iter {
            let (hash, id) = instance?;
            instance_to_card_id_map.insert(hash, id);
        }
    }

    // 3. Process the decks table.
    let mut decks_with_cards: Vec<(String, Vec<u32>)> = Vec::new();
    let mut stmt = conn.prepare("SELECT * FROM decks")?;
    let mut deck_rows = stmt.query([])?;

    while let Some(row) = deck_rows.next()? {
        let deck_hash: String = row.get(0)?;
        let mut card_ids: Vec<u32> = Vec::with_capacity(8);

        // Iterate through the 8 card_instance_hash columns.
        for i in 1..=8 {
            let instance_hash: String = row.get(i)?;
            // Use the map to find the corresponding card_type_id.
            // .expect() is used here assuming data integrity; an error would indicate
            // a deck references a card instance that doesn't exist.
            let card_id = instance_to_card_id_map.get(&instance_hash)
                .expect("Data integrity error: deck references a non-existent card instance.");
            card_ids.push(*card_id);
        }

        decks_with_cards.push((deck_hash, card_ids));
    }

    // 4. Return the result.
    Ok(decks_with_cards)
}
