use serde::Deserialize;

/// A type alias for a collection of battles, representing the top-level API response.
pub type BattleLog = Vec<Battle>;

/// Represents a single battle from the Clash Royale API.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Battle {
    /// The timestamp of when the battle took place.
    pub battle_time: String,
    /// A list of players on the primary team.
    pub team: Vec<PlayerInfo>,
    /// A list of players on the opposing team.
    pub opponent: Vec<PlayerInfo>,
    // Other fields from the API can be added here if needed.
    // Serde will ignore fields present in the JSON but not in the struct.
}

/// Represents information about a player in a battle.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerInfo {
    /// The player's unique tag.
    pub tag: String,
    /// The player's in-game name.
    pub name: String,
    /// The number of crowns the player earned in the battle.
    pub crowns: u8,
    /// The deck of cards the player used in the battle.
    pub cards: Vec<Card>,
    /// The player's trophy count at the start of the battle. Optional.
    pub starting_trophies: Option<i32>,
    /// The change in the player's trophies after the battle. Optional.
    pub trophy_change: Option<i32>,
    /// The hit points of the player's king tower. Optional.
    pub king_tower_hit_points: Option<u32>,
}

/// Represents a single card used by a player.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    /// The name of the card.
    pub name: String,
    /// The unique ID of the card.
    pub id: u32,
    /// The level of the card.
    pub level: u8,
    /// The elixir cost of the card. Optional.
    pub elixir_cost: Option<u8>,
    /// The evolution level of the card, if it is evolved. Optional.
    pub evolution_level: Option<u8>,
}
