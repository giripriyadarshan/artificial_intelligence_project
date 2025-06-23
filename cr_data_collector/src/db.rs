use crate::api_models::{BattleLog, Card};
use deadpool_sqlite::rusqlite::{params, Connection as RusqliteConnection, Result as RusqliteResult};
use deadpool_sqlite::Connection as DeadpoolConnection;
use sha2::{Digest, Sha256};

/// Initializes the application's database by creating necessary tables if they don't exist.
///
/// NOTE: If run against an existing database, this will not add new columns to existing
/// tables. Delete the old .db file to apply schema changes.
pub fn initialize_database(conn: &mut RusqliteConnection) -> RusqliteResult<()> {
    let tx = conn.transaction()?;

    // -- Create decks table (unchanged) --
    tx.execute(
        "CREATE TABLE IF NOT EXISTS decks (
            deck_hash       TEXT PRIMARY KEY,
            card_id_1       INTEGER NOT NULL, card_id_2       INTEGER NOT NULL,
            card_id_3       INTEGER NOT NULL, card_id_4       INTEGER NOT NULL,
            card_id_5       INTEGER NOT NULL, card_id_6       INTEGER NOT NULL,
            card_id_7       INTEGER NOT NULL, card_id_8       INTEGER NOT NULL
        )",
        [],
    )?;

    // -- Create battles table (with new nullable columns) --
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

    // -- Create new cards table --
    // This table will store details about each unique card encountered.
    tx.execute(
        "CREATE TABLE IF NOT EXISTS cards (
            id              INTEGER PRIMARY KEY,
            name            TEXT NOT NULL,
            elixir_cost     INTEGER
        )",
        [],
    )?;

    tx.commit()
}

/// Calculates a SHA-256 hash for a deck to create a unique identifier.
fn calculate_deck_hash(cards: &[Card]) -> (String, Vec<u32>) {
    let mut card_ids: Vec<u32> = cards.iter().map(|card| card.id).collect();
    card_ids.sort_unstable();
    let canonical_string = card_ids.iter().map(ToString::to_string).collect::<Vec<_>>().join(",");
    let mut hasher = Sha256::new();
    hasher.update(canonical_string.as_bytes());
    (format!("{:x}", hasher.finalize()), card_ids)
}

/// Saves a battle log and card details to the database asynchronously.
pub async fn save_battle_log(
    conn: &DeadpoolConnection,
    battle_log: BattleLog,
) -> Result<RusqliteResult<usize>, deadpool_sqlite::InteractError> {
    conn.interact(move |conn| {
        let tx = conn.transaction()?;
        let mut new_battles_count = 0;

        for battle in &battle_log {
            if battle.team.len() != 1 || battle.opponent.len() != 1
                || battle.team[0].cards.len() != 8 || battle.opponent[0].cards.len() != 8 {
                continue;
            }

            // -- Save unique card details --
            let all_cards = battle.team[0].cards.iter().chain(battle.opponent[0].cards.iter());
            for card in all_cards {
                // INSERT OR IGNORE is efficient; it does nothing if the card ID already exists.
                tx.execute(
                    "INSERT OR IGNORE INTO cards (id, name, elixir_cost) VALUES (?1, ?2, ?3)",
                    params![card.id, card.name, card.elixir_cost],
                )?;
            }

            // -- Process players and decks --
            let player_a = &battle.team[0];
            let player_b = &battle.opponent[0];
            let (player_a_deck_hash, player_a_sorted_ids) = calculate_deck_hash(&player_a.cards);
            let (player_b_deck_hash, player_b_sorted_ids) = calculate_deck_hash(&player_b.cards);

            tx.execute(
                "INSERT OR IGNORE INTO decks (deck_hash, card_id_1, card_id_2, card_id_3, card_id_4, card_id_5, card_id_6, card_id_7, card_id_8)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    player_a_deck_hash,
                    player_a_sorted_ids[0], player_a_sorted_ids[1], player_a_sorted_ids[2], player_a_sorted_ids[3],
                    player_a_sorted_ids[4], player_a_sorted_ids[5], player_a_sorted_ids[6], player_a_sorted_ids[7],
                ],
            )?;

            tx.execute(
                "INSERT OR IGNORE INTO decks (deck_hash, card_id_1, card_id_2, card_id_3, card_id_4, card_id_5, card_id_6, card_id_7, card_id_8)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    player_b_deck_hash,
                    player_b_sorted_ids[0], player_b_sorted_ids[1], player_b_sorted_ids[2], player_b_sorted_ids[3],
                    player_b_sorted_ids[4], player_b_sorted_ids[5], player_b_sorted_ids[6], player_b_sorted_ids[7],
                ],
            )?;

            // -- Insert battle record with new optional fields --
            let changes = tx.execute(
                "INSERT INTO battles (
                    battle_time,
                    player_a_tag, player_a_crowns, player_a_deck_hash, player_a_starting_trophies, player_a_trophy_change, player_a_king_tower_hit_points,
                    player_b_tag, player_b_crowns, player_b_deck_hash, player_b_starting_trophies, player_b_trophy_change, player_b_king_tower_hit_points
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    battle.battle_time,
                    player_a.tag,
                    player_a.crowns,
                    player_a_deck_hash,
                    player_a.starting_trophies,
                    player_a.trophy_change,
                    player_a.king_tower_hit_points,
                    player_b.tag,
                    player_b.crowns,
                    player_b_deck_hash,
                    player_b.starting_trophies,
                    player_b.trophy_change,
                    player_b.king_tower_hit_points,
                ],
            )?;
            new_battles_count += changes;
        }

        tx.commit()?;
        Ok(new_battles_count)
    })
        .await
}
