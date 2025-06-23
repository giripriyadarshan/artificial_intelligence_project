use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use futures::stream::StreamExt;
use deadpool_sqlite::{Config as DeadpoolConfig, Runtime};
use tokio::sync::Mutex;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

// Declare the modules
mod config;
pub mod api_models;
pub mod api_client;
pub mod db;

// --- Constants for the concurrent loop ---
const BATCH_SIZE: usize = 50; // Number of tags to process from the queue in one go.
const CONCURRENT_REQUESTS: usize = 10; // Max number of API requests to have in flight at once.

#[tokio::main]
async fn main() {
    // Initialize Logging
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("🚀 Starting High-Concurrency Clash Royale Data Collector...");

    // --- Initialize clients and configuration ---
    let config = config::Config::from_env();
    
    // Create a deadpool for SQLite connections using the correct API.
    let pool_cfg = DeadpoolConfig::new(&config.database_url);
    let pool = pool_cfg.create_pool(Runtime::Tokio1).expect("Failed to create pool.");

    // Get an initial connection to set up the database schema.
    let conn = pool.get().await.expect("Failed to get initial db connection");
    conn.interact(|conn| db::initialize_database(conn))
        .await
        .expect("Database interaction failed")
        .expect("Failed to initialize database schema");
    
    let http_client = Arc::new(reqwest::Client::new());
    
    info!("✅ Configuration and clients initialized successfully.");

    // --- Set up shared state for concurrency ---
    let tags_to_process = Arc::new(Mutex::new(VecDeque::new()));
    let processed_tags = Arc::new(Mutex::new(HashSet::new()));

    // Seed the queue
    {
        let mut queue = tags_to_process.lock().await;
        queue.push_back("#RVCQ2CQGJ".to_string());
        queue.push_back("#VCQUY9Y8U".to_string());
    }
    info!("Seeded queue with initial tags. Starting main processing loop...");

    // --- Main Concurrent Loop ---
    loop {
        let mut batch_of_tags = Vec::with_capacity(BATCH_SIZE);
        
        // Lock the queue, drain a batch of tags, then immediately release the lock.
        {
            let mut queue_guard = tags_to_process.lock().await;
            
            // ** THE FIX IS HERE **
            // 1. First, get the number of items to drain (immutable borrow).
            let drain_count = std::cmp::min(BATCH_SIZE, queue_guard.len());
            // 2. Then, use that number to drain (mutable borrow).
            batch_of_tags.extend(queue_guard.drain(..drain_count));
        }

        if batch_of_tags.is_empty() {
            info!("Queue is empty, waiting a moment to see if new tags appear...");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            
            // If the queue is still empty after waiting, we can shut down.
            if tags_to_process.lock().await.is_empty() {
                info!("🏁 Queue is still empty. Shutting down.");
                break;
            }
            continue;
        }

        info!("Processing a batch of {} tags...", batch_of_tags.len());

        // Process the entire batch concurrently.
        futures::stream::iter(batch_of_tags)
            .for_each_concurrent(CONCURRENT_REQUESTS, |tag| {
                // Clone Arcs and Pool for use in the async task.
                let pool = pool.clone();
                let http_client = http_client.clone();
                let api_key = config.api_key.clone();
                let tags_to_process = tags_to_process.clone();
                let processed_tags = processed_tags.clone();

                async move {
                    // Skip if another concurrent task has already processed this tag.
                    if processed_tags.lock().await.contains(&tag) {
                        return;
                    }

                    info!("-> Fetching tag: {}", tag);
                    let db_conn = match pool.get().await {
                        Ok(conn) => conn,
                        Err(e) => {
                            error!("Failed to get DB connection from pool: {}", e);
                            return;
                        }
                    };

                    match api_client::fetch_battle_log(&http_client, &api_key, &tag).await {
                        Ok(battle_log) => {
                            // Discover new tags before saving.
                            let mut discovered_tags = Vec::new();
                            for battle in &battle_log {
                                if let Some(opponent) = battle.opponent.get(0) {
                                    discovered_tags.push(opponent.tag.clone());
                                }
                            }
                            
                            // Save battles to DB, handling the nested Result.
                            match db::save_battle_log(&db_conn, battle_log).await {
                                Ok(Ok(count)) => info!("   ✅ Saved {} new battles for tag {}", count, tag),
                                Ok(Err(e)) => error!("   ❌ DB (rusqlite) Error for tag {}: {}", tag, e),
                                Err(e) => error!("   ❌ DB (deadpool) Error for tag {}: {}", tag, e),
                            }

                            // Add newly discovered tags to the shared queue.
                            let mut queue_guard = tags_to_process.lock().await;
                            let processed_guard = processed_tags.lock().await;
                            for discovered_tag in discovered_tags {
                                if !processed_guard.contains(&discovered_tag) && !queue_guard.contains(&discovered_tag) {
                                    queue_guard.push_back(discovered_tag);
                                }
                            }
                        }
                        Err(e) => error!("   ❌ API Error for tag {}: {}", tag, e),
                    }
                    
                    // Mark tag as processed.
                    processed_tags.lock().await.insert(tag);
                }
            })
            .await;
        
        info!("Finished processing batch. Processed tags count: {}", processed_tags.lock().await.len());
    }
}
