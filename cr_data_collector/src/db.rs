use crate::api_models::{BattleLog, Card};
use deadpool_sqlite::rusqlite::{params, Connection as RusqliteConnection, Result as RusqliteResult};
use deadpool_sqlite::Connection as DeadpoolConnection;
use sha2::{Digest, Sha256};

/// Initializes the application's database with a normalized schema.
///
/// This function creates the four necessary tables for storing card metadata, unique
/// card instances, unique decks, and battle logs.
///
/// NOTE: If run against an existing database with the old schema, this will not
/// perform a migration. Delete the old .db file to apply these schema changes.
pub fn initialize_database(conn: &mut RusqliteConnection) -> RusqliteResult<()> {
    let tx = conn.transaction()?;

    // -- Create card_metadata table for static card info --
    tx.execute(
        "CREATE TABLE IF NOT EXISTS card_metadata (
            id              INTEGER PRIMARY KEY,
            name            TEXT NOT NULL,
            elixir_cost     INTEGER
        )",
        [],
    )?;

    // -- Create card_instances table for unique card variations (id + level + evolution) --
    tx.execute(
        "CREATE TABLE IF NOT EXISTS card_instances (
            instance_hash   TEXT PRIMARY KEY,
            card_type_id    INTEGER NOT NULL,
            level           INTEGER NOT NULL,
            evolution_level INTEGER,
            FOREIGN KEY (card_type_id) REFERENCES card_metadata (id)
        )",
        [],
    )?;

    // -- Create decks table, which now references unique card_instances --
    tx.execute(
        "CREATE TABLE IF NOT EXISTS decks (
            deck_hash               TEXT PRIMARY KEY,
            card_instance_hash_1    TEXT NOT NULL, card_instance_hash_2    TEXT NOT NULL,
            card_instance_hash_3    TEXT NOT NULL, card_instance_hash_4    TEXT NOT NULL,
            card_instance_hash_5    TEXT NOT NULL, card_instance_hash_6    TEXT NOT NULL,
            card_instance_hash_7    TEXT NOT NULL, card_instance_hash_8    TEXT NOT NULL,
            FOREIGN KEY (card_instance_hash_1) REFERENCES card_instances (instance_hash),
            FOREIGN KEY (card_instance_hash_2) REFERENCES card_instances (instance_hash),
            FOREIGN KEY (card_instance_hash_3) REFERENCES card_instances (instance_hash),
            FOREIGN KEY (card_instance_hash_4) REFERENCES card_instances (instance_hash),
            FOREIGN KEY (card_instance_hash_5) REFERENCES card_instances (instance_hash),
            FOREIGN KEY (card_instance_hash_6) REFERENCES card_instances (instance_hash),
            FOREIGN KEY (card_instance_hash_7) REFERENCES card_instances (instance_hash),
            FOREIGN KEY (card_instance_hash_8) REFERENCES card_instances (instance_hash)
        )",
        [],
    )?;

    // -- Create battles table (schema unchanged, but deck_hash meaning is now more specific) --
    tx.execute(
        "CREATE TABLE IF NOT EXISTS battles (
            id                              INTEGER PRIMARY KEY AUTOINCREMENT,
            battle_time                     TEXT NOT NULL,
            player_a_tag                    TEXT NOT NULL,
            player_a_crowns                 INTEGER NOT NULL,
            player_a_deck_hash              TEXT NOT NULL,
            player_a_starting_trophies      INTEGER,
            player_a_trophy_change          INTEGER,
            player_a_king_tower_hit_points  INTEGER,
            player_b_tag                    TEXT NOT NULL,
            player_b_crowns                 INTEGER NOT NULL,
            player_b_deck_hash              TEXT NOT NULL,
            player_b_starting_trophies      INTEGER,
            player_b_trophy_change          INTEGER,
            player_b_king_tower_hit_points  INTEGER,
            FOREIGN KEY (player_a_deck_hash) REFERENCES decks (deck_hash),
            FOREIGN KEY (player_b_deck_hash) REFERENCES decks (deck_hash)
        )",
        [],
    )?;

    // Drop the old, now-redundant `cards` table if it exists
    tx.execute("DROP TABLE IF EXISTS cards", [])?;

    tx.commit()
}

/// Creates a unique hash for a specific card instance (type, level, and evolution).
fn calculate_card_instance_hash(card: &Card) -> String {
    // Treat None evolution level as 0 for hashing purposes.
    let evolution_level = card.evolution_level.unwrap_or(0);
    let canonical_string = format!("{}-{}-{}", card.id, card.level, evolution_level);
    let mut hasher = Sha256::new();
    hasher.update(canonical_string.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Creates a unique hash for a deck, based on the sorted hashes of its 8 unique card instances.
fn calculate_deck_hash(card_instance_hashes: &mut [String]) -> String {
    card_instance_hashes.sort_unstable();
    let canonical_string = card_instance_hashes.join(",");
    let mut hasher = Sha256::new();
    hasher.update(canonical_string.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Saves a battle log and its associated card data to the normalized database schema.
pub async fn save_battle_log(
    conn: &DeadpoolConnection,
    battle_log: BattleLog,
) -> Result<RusqliteResult<usize>, deadpool_sqlite::InteractError> {
    conn.interact(move |conn| {
        let tx = conn.transaction()?;
        let mut new_battles_count = 0;

        for battle in &battle_log {
            // Ensure we only process 1v1 battles with full 8-card decks.
            if battle.team.len() != 1 || battle.opponent.len() != 1
                || battle.team[0].cards.len() != 8 || battle.opponent[0].cards.len() != 8 {
                continue;
            }

            // -- Step 1: Process all 16 cards to save their metadata and instances --
            let all_cards = battle.team[0].cards.iter().chain(battle.opponent[0].cards.iter());
            for card in all_cards {
                // Save static card info (name, elixir). `OR IGNORE` is efficient.
                tx.execute(
                    "INSERT OR IGNORE INTO card_metadata (id, name, elixir_cost) VALUES (?1, ?2, ?3)",
                    params![card.id, card.name, card.elixir_cost],
                )?;
                // Save unique card instance (id + level + evolution).
                tx.execute(
                    "INSERT OR IGNORE INTO card_instances (instance_hash, card_type_id, level, evolution_level) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        calculate_card_instance_hash(card),
                        card.id,
                        card.level,
                        card.evolution_level
                    ],
                )?;
            }

            // -- Step 2: Process Player A's deck --
            let player_a = &battle.team[0];
            let mut player_a_instance_hashes: Vec<String> = player_a.cards.iter().map(calculate_card_instance_hash).collect();
            let player_a_deck_hash = calculate_deck_hash(&mut player_a_instance_hashes);
            tx.execute(
                "INSERT OR IGNORE INTO decks (deck_hash, card_instance_hash_1, card_instance_hash_2, card_instance_hash_3, card_instance_hash_4, card_instance_hash_5, card_instance_hash_6, card_instance_hash_7, card_instance_hash_8)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    player_a_deck_hash,
                    player_a_instance_hashes[0], player_a_instance_hashes[1], player_a_instance_hashes[2], player_a_instance_hashes[3],
                    player_a_instance_hashes[4], player_a_instance_hashes[5], player_a_instance_hashes[6], player_a_instance_hashes[7],
                ],
            )?;

            // -- Step 3: Process Player B's deck --
            let player_b = &battle.opponent[0];
            let mut player_b_instance_hashes: Vec<String> = player_b.cards.iter().map(calculate_card_instance_hash).collect();
            let player_b_deck_hash = calculate_deck_hash(&mut player_b_instance_hashes);
            tx.execute(
                "INSERT OR IGNORE INTO decks (deck_hash, card_instance_hash_1, card_instance_hash_2, card_instance_hash_3, card_instance_hash_4, card_instance_hash_5, card_instance_hash_6, card_instance_hash_7, card_instance_hash_8)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    player_b_deck_hash,
                    player_b_instance_hashes[0], player_b_instance_hashes[1], player_b_instance_hashes[2], player_b_instance_hashes[3],
                    player_b_instance_hashes[4], player_b_instance_hashes[5], player_b_instance_hashes[6], player_b_instance_hashes[7],
                ],
            )?;

            // -- Step 4: Insert the battle record with new optional fields --
            let changes = tx.execute(
                "INSERT INTO battles (
                    battle_time,
                    player_a_tag, player_a_crowns, player_a_deck_hash, player_a_starting_trophies, player_a_trophy_change, player_a_king_tower_hit_points,
                    player_b_tag, player_b_crowns, player_b_deck_hash, player_b_starting_trophies, player_b_trophy_change, player_b_king_tower_hit_points
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    battle.battle_time,
                    player_a.tag, player_a.crowns, player_a_deck_hash, player_a.starting_trophies, player_a.trophy_change, player_a.king_tower_hit_points,
                    player_b.tag, player_b.crowns, player_b_deck_hash, player_b.starting_trophies, player_b.trophy_change, player_b.king_tower_hit_points,
                ],
            )?;
            new_battles_count += changes;
        }

        tx.commit()?;
        Ok(new_battles_count)
    })
        .await
}
