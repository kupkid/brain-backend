use anyhow::Result;
use axum::Router;
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod config;
mod db;
mod memory;
mod project;
mod provider;
mod run;
mod vault;
mod workspace;

use config::AppConfig;
use api::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "brain_backend=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env()?;

    // Ensure data directory exists
    std::fs::create_dir_all(&config.data_dir)?;

    // Initialize database
    let db_path = config.data_dir.join("brain.db");
    let conn = db::init_db(&db_path)?;

    // Initialize vault with master key
    let master_key = match &config.master_key_hex {
        Some(hex_key) => {
            let bytes = hex::decode(hex_key)?;
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            key
        }
        None => {
            // Generate random key for first run
            use rand::RngCore;
            let mut key = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut key);
            tracing::warn!("generated random master key — set BRAIN_MASTER_KEY env for persistence");
            key
        }
    };

    let vault = vault::VaultRepository::new(&conn);
    vault.init(&master_key)?;

    // Ensure vec0 tables exist for configured dimensions
    db::ensure_vec_table(&conn, config.embedding_provider.dimensions as i32)?;

    // Create embedding collection if none exists
    let collection_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM embedding_collections",
        [],
        |r| r.get(0),
    )?;
    if collection_count == 0 {
        use crate::db::ids;
        let uuid = ids::new_uuid_blob();
        conn.execute(
            "INSERT INTO embedding_collections (uuid, model_name, dimensions, distance_metric, active)
             VALUES (?1, ?2, ?3, ?4, 1)",
            rusqlite::params![
                uuid,
                config.embedding_provider.model,
                config.embedding_provider.dimensions as i64,
                "cosine",
            ],
        )?;
        tracing::info!("created default embedding collection");
    }

    // Create application state
    let state = Arc::new(AppState {
        config: config.clone(),
        conn: Mutex::new(conn),
    });

    // Build router
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = api::create_router(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = format!("{}:{}", config.listen_addr, config.listen_port);
    tracing::info!("starting brain-backend on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
