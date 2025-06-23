use std::env;

/// Holds the application's configuration values.
pub struct Config {
    pub api_key: String,
    pub database_url: String,
}

impl Config {
    /// Creates a new Config instance by loading values from environment variables.
    ///
    /// This function will load variables from a .env file if it exists in the
    /// project root. It will panic if any of the required environment variables
    /// are not set.
    pub fn from_env() -> Self {
        // Load environment variables from the .env file.
        // .ok() silently ignores errors, which is fine if the file doesn't exist.
        dotenvy::dotenv().ok();

        // Load CLASH_ROYALE_API_KEY, panicking if it's not set.
        let api_key = env::var("CLASH_ROYALE_API_KEY")
            .expect("CLASH_ROYALE_API_KEY must be set in your .env file");

        // Load DATABASE_URL, panicking if it's not set.
        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set in your .env file");

        Config {
            api_key,
            database_url,
        }
    }
}

