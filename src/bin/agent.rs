use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use brain_backend::agent::{AgentLoop, AgentConfig};
use brain_backend::agent::tools;
use brain_backend::provider::cohere_llm::CohereLlm;
use brain_backend::provider::cohere_embedding::CohereEmbedding;
use brain_backend::provider::llm::LlmProvider;
use brain_backend::provider::embedding::EmbeddingProvider;
use brain_backend::agent::agent_loop::AgentMessage;
use brain_backend::agent::tool_trait::ToolImportance;
use brain_backend::db;
use colored::*;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "brain_backend=info".into()),
        )
        .init();

    let api_key = std::env::var("COHERE_API_KEY")
        .expect("COHERE_API_KEY env var is required");

    let data_dir = std::env::var("BRAIN_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("~/.brain"));

    std::fs::create_dir_all(&data_dir).ok();
    let db_path = data_dir.join("brain.db");

    println!("{}", "=== Brain Agent ===".cyan().bold());
    println!("Data dir: {}", data_dir.display());

    let conn = Arc::new(Mutex::new(db::init_db(&db_path).expect("failed to init db")));
    db::ensure_vec_table(&conn.lock().unwrap(), 1024).ok();

    let llm = Arc::new(CohereLlm::new(api_key.clone(), None, None));
    let embedding = Arc::new(CohereEmbedding::new(api_key, None));

    print!("Health check... ");
    if llm.health_check().await && embedding.health_check().await {
        println!("{}", "OK".green());
    } else {
        println!("{}", "FAILED".red());
        std::process::exit(1);
    }

    let config = AgentConfig::from_env();
    let run_id = 1i64;
    let toolbox = tools::build_default_tools(
        &conn, run_id,
        config.workspace_dir.clone(), config.tool_timeout_seconds,
    );

    println!("Tools: {}\n", toolbox.names().join(", ").dimmed());

    let agent = AgentLoop::new(
        llm, embedding, conn, toolbox, config, run_id,
    );

    let mut history: Vec<AgentMessage> = Vec::new();
    let stdin = std::io::stdin();

    loop {
        print!("{} ", ">".green().bold());
        use std::io::Write;
        std::io::stdout().flush().ok();

        let mut input = String::new();
        if stdin.read_line(&mut input).unwrap_or(0) == 0 { break; }
        let input = input.trim();
        if input.is_empty() { continue; }
        if matches!(input, "/quit" | "/exit" | "/q") { println!("Bye!"); break; }
        if input == "/history" {
            for msg in &history {
                let role = if msg.role == "user" { "You".blue() } else { "Agent".green() };
                println!("  {}: {}", role, msg.content);
            }
            continue;
        }
        if input == "/clear" { history.clear(); println!("Cleared."); continue; }

        let response = agent.process_message(input, &history).await;

        println!("\n{}", response.content);
        if !response.tool_results.is_empty() {
            println!("\n{}", "Tools:".dimmed());
            for tr in &response.tool_results {
                let status = if tr.output.result.is_null() { "err".red() } else { "ok".green() };
                println!("  {} {} — {}", status, tr.name, tr.output.result.to_string().dimmed());
            }
        }
        println!("{}\n", format!("({} tokens)", response.tokens_used).dimmed());

        history.push(AgentMessage {
            role: "user".to_string(),
            content: input.to_string(),
            importance: ToolImportance::High,
        });
        history.push(AgentMessage {
            role: "assistant".to_string(),
            content: response.content,
            importance: ToolImportance::Normal,
        });
    }
}
