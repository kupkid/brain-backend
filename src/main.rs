use anyhow::Result;
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use brain_backend::agent::EventBus;
use brain_backend::api::AppState;
use brain_backend::config::AppConfig;
use brain_backend::db;
use brain_backend::provider::cohere_embedding::CohereEmbedding;
use brain_backend::provider::embedding::EmbeddingProvider;
use brain_backend::provider::llm::LlmProvider;
use brain_backend::settings::providers::ProvidersRepository;
use brain_backend::vault;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "brain_backend=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env()?;
    std::fs::create_dir_all(&config.data_dir)?;

    let db_path = config.data_dir.join("brain.db");
    let conn = db::init_db(&db_path)?;

    let passphrase = std::env::var("BRAIN_VAULT_PASSPHRASE")
        .map_err(|_| anyhow::anyhow!("BRAIN_VAULT_PASSPHRASE env var is required"))?;

    let vault = vault::VaultRepository::new(&conn);
    let master_key = match vault.get_active_master_key_version()? {
        Some(_version) => {
            let material = vault.unlock(passphrase.as_bytes())?;
            material.key
        }
        None => {
            let material = vault.init(passphrase.as_bytes())?;
            tracing::info!("vault initialized for the first time");
            material.key
        }
    };

    db::ensure_vec_table(&conn, config.embedding_provider.dimensions as i32)?;

    let collection_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM embedding_collections", [], |r| {
            r.get(0)
        })?;
    if collection_count == 0 {
        use brain_backend::db::ids;
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

    // Create embedding provider (always Cohere for now)
    let emb_key = std::env::var("COHERE_API_KEY")
        .or_else(|_| std::env::var("LLM_API_KEY"))
        .expect("COHERE_API_KEY or LLM_API_KEY required for embeddings");
    let embedding: Arc<dyn EmbeddingProvider> = Arc::new(CohereEmbedding::new(emb_key, None));

    // Create LLM factory — reads provider from DB (providers table)
    let llm_factory: brain_backend::ws_agent::LlmFactory = Box::new(
        move |conn: &rusqlite::Connection, master_key: &[u8; 32], tools: serde_json::Value| {
            let providers_repo = ProvidersRepository::new(conn);
            // Try default provider first, then first enabled provider
            let provider = providers_repo
                .get_default()
                .ok()
                .flatten()
                .or_else(|| {
                    providers_repo
                        .list()
                        .ok()
                        .and_then(|ps| ps.into_iter().find(|p| p.enabled))
                });

            match provider {
                Some(p) => {
                    let api_key = providers_repo
                        .get_api_key(master_key, p.id)
                        .ok()
                        .flatten()
                        .unwrap_or_default();
                    tracing::info!(
                        "WS agent LLM: provider='{}' type='{}' base_url='{}'",
                        p.name,
                        p.provider_type,
                        p.base_url
                    );
                    match p.provider_type.as_str() {
                        "openai" | "openai_compat" => {
                            let provider = brain_backend::provider::openai_compat::OpenAiCompatLlm::new(
                                api_key, "gpt-4o".to_string(), p.base_url,
                            );
                            Arc::new(provider.with_tools(tools)) as Arc<dyn LlmProvider>
                        }
                        "cohere" => {
                            let provider = brain_backend::provider::cohere_llm::CohereLlm::new(
                                api_key, None, None,
                            );
                            Arc::new(provider.with_tools(tools)) as Arc<dyn LlmProvider>
                        }
                        _ => {
                            // Default to OpenAI-compatible
                            let provider = brain_backend::provider::openai_compat::OpenAiCompatLlm::new(
                                api_key, "gpt-4o".to_string(), p.base_url,
                            );
                            Arc::new(provider.with_tools(tools)) as Arc<dyn LlmProvider>
                        }
                    }
                }
                None => {
                    tracing::warn!("no provider configured — falling back to env vars");
                    // Fallback to env vars
                    let api_key = std::env::var("COHERE_API_KEY")
                        .or_else(|_| std::env::var("LLM_API_KEY"))
                        .unwrap_or_default();
                    let provider = brain_backend::provider::cohere_llm::CohereLlm::new(
                        api_key, None, None,
                    );
                    Arc::new(provider.with_tools(tools)) as Arc<dyn LlmProvider>
                }
            }
        },
    );

    let api_key = std::env::var("BRAIN_API_KEY").ok();

    let state = Arc::new(AppState {
        config: config.clone(),
        conn: Mutex::new(conn),
        master_key: Mutex::new(master_key),
        event_bus: Arc::new(EventBus::new(1024)),
        llm_factory: Arc::new(llm_factory),
        embedding,
        api_key,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = brain_backend::api::create_router(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = format!("{}:{}", config.listen_addr, config.listen_port);
    tracing::info!("starting brain-backend on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
