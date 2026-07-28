use anyhow::Result;
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
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "brain_backend=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env()?;
    std::fs::create_dir_all(&config.data_dir)?;

    // Initialize database
    let db_path = config.data_dir.join("brain.db");
    let conn = db::init_db(&db_path)?;

    // Initialize vault with passphrase
    let passphrase = std::env::var("BRAIN_VAULT_PASSPHRASE")
        .map_err(|_| anyhow::anyhow!("BRAIN_VAULT_PASSPHRASE env var is required"))?;

    let vault = vault::VaultRepository::new(&conn);
    let master_key = match vault.get_active_master_key_version()? {
        Some(_version) => {
            // Subsequent run: unlock existing vault
            let material = vault.unlock(passphrase.as_bytes())?;
            material.key
        }
        None => {
            // First run: initialize vault
            let material = vault.init(passphrase.as_bytes())?;
            tracing::info!("vault initialized for the first time");
            material.key
        }
    };

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
        master_key: Mutex::new(master_key),
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
