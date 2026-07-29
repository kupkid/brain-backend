use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use brain_backend::agent::{AgentLoop, AgentConfig};
use brain_backend::agent::tools;
use brain_backend::provider::cohere_llm::CohereLlm;
use brain_backend::provider::openai_compat::OpenAiCompatLlm;
use brain_backend::provider::cohere_embedding::CohereEmbedding;
use brain_backend::provider::embedding::EmbeddingProvider;
use brain_backend::provider::llm::LlmProvider;
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

    let data_dir = std::env::var("BRAIN_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("~/.brain"));

    std::fs::create_dir_all(&data_dir).ok();
    let db_path = data_dir.join("brain.db");

    println!("{}", "=== Brain Agent ===".cyan().bold());
    println!("Data dir: {}", data_dir.display());

    let conn = Arc::new(Mutex::new(db::init_db(&db_path).expect("failed to init db")));
    db::ensure_vec_table(&conn.lock().unwrap(), 1024).ok();

    // Ensure default embedding collection
    {
        let c = conn.lock().unwrap();
        let count: i64 = c.query_row("SELECT COUNT(*) FROM embedding_collections", [], |r| r.get(0)).unwrap_or(0);
        if count == 0 {
            use brain_backend::db::ids;
            let uuid = ids::new_uuid_blob();
            c.execute(
                "INSERT INTO embedding_collections (uuid, model_name, dimensions, distance_metric, active)
                 VALUES (?1, 'embed-multilingual-v3.0', 1024, 'cosine', 1)",
                [uuid],
            ).ok();
        }
    }

    // Create a run for this session
    let run_id = {
        let c = conn.lock().unwrap();
        let uuid = brain_backend::db::ids::new_uuid_blob();
        c.execute(
            "INSERT INTO runs (uuid, agent_name, goal, context_json, status)
             VALUES (?1, 'cli-agent', 'interactive session', '{}', 'running')",
            [uuid],
        ).expect("failed to create run");
        c.last_insert_rowid()
    };

    let config = AgentConfig::from_env();
    let workspace = config.workspace_dir.clone();

    let toolbox = tools::build_default_tools(
        &conn, run_id,
        workspace.clone(), config.tool_timeout_seconds,
    );

    // Create LLM provider based on env vars
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "cohere".to_string());
    let llm: Arc<dyn LlmProvider> = match provider.as_str() {
        "openai_compat" => {
            let api_key = std::env::var("LLM_API_KEY").expect("LLM_API_KEY required for openai_compat");
            let model = std::env::var("LLM_MODEL").expect("LLM_MODEL required for openai_compat");
            let base_url = std::env::var("LLM_BASE_URL").expect("LLM_BASE_URL required for openai_compat");
            println!("Provider: OpenAI-compatible ({}, {})", model, base_url);
            Arc::new(OpenAiCompatLlm::new(api_key, model, base_url).with_tools(toolbox.schema()))
        }
        _ => {
            let api_key = std::env::var("COHERE_API_KEY").expect("COHERE_API_KEY required");
            println!("Provider: Cohere (command-a-plus-05-2026)");
            Arc::new(CohereLlm::new(api_key.clone(), None, None).with_tools(toolbox.schema()))
        }
    };

    // Embedding provider (always Cohere for now)
    let emb_key = std::env::var("COHERE_API_KEY")
        .or_else(|_| std::env::var("LLM_API_KEY"))
        .expect("COHERE_API_KEY or LLM_API_KEY required for embeddings");
    let embedding = Arc::new(CohereEmbedding::new(emb_key, None));

    print!("Health check... ");
    let llm_ok = llm.health_check().await;
    let emb_ok = embedding.health_check().await;
    if llm_ok && emb_ok {
        println!("{}", "OK".green());
    } else {
        if !llm_ok { println!("LLM {}", "FAILED".red()); }
        if !emb_ok { println!("Embedding {}", "FAILED".red()); }
        std::process::exit(1);
    }

    println!("Run: {run_id}");
    println!("Tools: {}\n", toolbox.names().join(", ").dimmed());

    let agent = AgentLoop::new(
        llm, embedding, conn, toolbox, config, run_id,
    );

    let mut history: Vec<AgentMessage> = Vec::new();
    let stdin = std::io::stdin();

    // Detect if stdin is a pipe (non-interactive)
    use std::io::IsTerminal;
    let is_tty = stdin.is_terminal();

    if !is_tty {
        // Piped input: read everything at once
        let mut input = String::new();
        std::io::Read::read_to_string(&mut stdin.lock(), &mut input).ok();
        let input = input.trim();
        if input.is_empty() { return; }

        let response = agent.process_message(input, &history).await;
        println!("\n{}", response.content);
        if !response.tool_results.is_empty() {
            println!("\n{}", "Tools:".dimmed());
            for tr in &response.tool_results {
                let status = if tr.output.result.is_null() { "err".red() } else { "ok".green() };
                let summary = tr.output.result.to_string();
                let short = if summary.len() > 120 { format!("{}...", &summary[..120]) } else { summary };
                println!("  {} {} — {}", status, tr.name, short.dimmed());
            }
        }
        println!("{}\n", format!("({} tokens)", response.tokens_used).dimmed());
        return;
    }

    // Interactive mode: line by line
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
                let summary = tr.output.result.to_string();
                let short = if summary.len() > 120 { format!("{}...", &summary[..120]) } else { summary };
                println!("  {} {} — {}", status, tr.name, short.dimmed());
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
