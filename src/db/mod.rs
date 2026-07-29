use rusqlite::Connection;
use std::path::Path;
use tracing::info;

pub mod ids;

static MIGRATION_SQL: &str = include_str!("../../migrations/001_init.sql");
static MIGRATION_TODOS: &str = include_str!("../../migrations/002_agent_todos.sql");
static MIGRATION_PROVIDER: &str = include_str!("../../migrations/003_provider_settings.sql");
static MIGRATION_PROVIDERS: &str = include_str!("../../migrations/004_providers.sql");

pub fn init_db(db_path: &Path) -> anyhow::Result<Connection> {
    // Register sqlite-vec as auto-extension BEFORE opening any connection.
    // The transmute through black_box prevents LTO from stripping the symbol.
    let init_fn = std::hint::black_box(sqlite_vec::sqlite3_vec_init);
    #[allow(clippy::missing_transmute_annotations)]
    unsafe {
        rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(init_fn as *const ())));
    }

    let conn = Connection::open(db_path)?;

    // Run WAL and performance pragmas
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
    conn.execute_batch("PRAGMA cache_size = -64000;")?; // 64MB page cache
    conn.execute_batch("PRAGMA temp_store = MEMORY;")?;

    // Apply DDL
    conn.execute_batch(MIGRATION_SQL)?;
    conn.execute_batch(MIGRATION_TODOS)?;
    conn.execute_batch(MIGRATION_PROVIDER)?;
    conn.execute_batch(MIGRATION_PROVIDERS)?;
    info!("database initialized at {}", db_path.display());

    // Verify vec0 is available
    match conn.query_row("SELECT vec_version()", [], |r| r.get::<_, String>(0)) {
        Ok(version) => info!("sqlite-vec version: {}", version),
        Err(e) => {
            anyhow::bail!("sqlite-vec not loaded: {}", e);
        }
    }

    // Verify FTS5 is available
    conn.execute_batch("CREATE VIRTUAL TABLE IF NOT EXISTS _fts5_test USING fts5(content);")?;
    conn.execute_batch("DROP TABLE IF EXISTS _fts5_test;")?;
    info!("FTS5 verified");

    Ok(conn)
}

pub fn ensure_vec_table(conn: &Connection, dimensions: i32) -> anyhow::Result<()> {
    let valid = matches!(dimensions, 384 | 768 | 1024 | 1536 | 3072);
    anyhow::ensure!(valid, "dimensions {} not in whitelist", dimensions);

    let table_name = format!("vec_mem_{}", dimensions);
    let distance = "cosine";

    conn.execute_batch(&format!(
        "CREATE VIRTUAL TABLE IF NOT EXISTS {table_name} USING vec0(
            vector_id INTEGER PRIMARY KEY,
            embedding float[{dimensions}] distance_metric={distance}
        );"
    ))?;

    info!(
        "ensured vec0 table: {} (float[{}], {})",
        table_name, dimensions, distance
    );
    Ok(())
}
