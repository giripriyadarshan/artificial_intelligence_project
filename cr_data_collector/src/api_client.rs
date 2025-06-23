use crate::api_models::BattleLog;

const API_BASE_URL: &str = "https://api.clashroyale.com/v1";

/// Fetches the battle log for a given player tag from the Clash Royale API.
///
/// # Arguments
///
/// * `client` - A shared reference to a `reqwest::Client`.
/// * `api_key` - The API key for authorization.
/// * `player_tag` - The player tag, which may include a leading '#'.
///
/// # Returns
///
/// A `Result` containing either the `BattleLog` on success or a `reqwest::Error` on failure.
pub async fn fetch_battle_log(
    client: &reqwest::Client,
    api_key: &str,
    player_tag: &str,
) -> Result<BattleLog, reqwest::Error> {
    // Player tags in the API URL must be URL-encoded.
    // The '#' character, in particular, must be replaced with '%23'.
    let encoded_player_tag = player_tag.replace('#', "%23");

    let request_url = format!(
        "{}/players/{}/battlelog",
        API_BASE_URL, encoded_player_tag
    );

    println!("Fetching data from: {}", request_url);

    let response = client
        .get(&request_url)
        .bearer_auth(api_key) // Add the "Authorization: Bearer <key>" header.
        .send()
        .await?;

    // Check if the request was successful (e.g., status code 200 OK).
    // If not, this will return an Err variant with the status code.
    let response = response.error_for_status()?;

    // Deserialize the JSON response body into our BattleLog struct.
    let battle_log = response.json::<BattleLog>().await?;

    Ok(battle_log)
}

