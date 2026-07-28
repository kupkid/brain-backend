use rusqlite::Connection;
use std::path::Path;
use tracing::info;

pub mod ids;

static MIGRATION_SQL: &str = include_str!("../../migrations/001_init.sql");

pub fn init_db(db_path: &Path) -> anyhow::Result<Connection> {
    let conn = Connection::open(db_path)?;

    // Register sqlite-vec as auto-extension BEFORE any queries
    unsafe {
        rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    }

    // Run WAL and performance pragmas
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
    conn.execute_batch("PRAGMA cache_size = -64000;")?; // 64MB page cache
    conn.execute_batch("PRAGMA temp_store = MEMORY;")?;

    // Apply DDL
    conn.execute_batch(MIGRATION_SQL)?;
    info!("database initialized at {}", db_path.display());

    // Verify vec0 is available
    let version: String = conn.query_row("SELECT vec_version()", [], |r| r.get(0))?;
    info!("sqlite-vec version: {}", version);

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

    info!("ensured vec0 table: {} (float[{}], {})", table_name, dimensions, distance);
    Ok(())
}
