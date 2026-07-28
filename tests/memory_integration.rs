use rusqlite::Connection;
use std::sync::Once;

static INIT: Once = Once::new();

fn setup_db() -> Connection {
    INIT.call_once(|| {
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }
    });

    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();

    let ddl = include_str!("../migrations/001_init.sql");
    conn.execute_batch(ddl).unwrap();

    // Create test projects
    conn.execute(
        "INSERT INTO projects (uuid, name, root_path) VALUES (?1, 'test-project-1', '/tmp/test1')",
        [brain_backend::db::ids::new_uuid_blob()],
    ).unwrap();
    conn.execute(
        "INSERT INTO projects (uuid, name, root_path) VALUES (?1, 'test-project-2', '/tmp/test2')",
        [brain_backend::db::ids::new_uuid_blob()],
    ).unwrap();

    // Create test run (id=1)
    conn.execute(
        "INSERT INTO runs (uuid, agent_name, goal, project_id) VALUES (?1, 'test-agent', 'test goal', 1)",
        [brain_backend::db::ids::new_uuid_blob()],
    ).unwrap();

    // Create default embedding collection for tests
    conn.execute(
        "INSERT INTO embedding_collections (uuid, model_name, dimensions, distance_metric)
         VALUES (?1, 'test-model', 1024, 'cosine')",
        [brain_backend::db::ids::new_uuid_blob()],
    ).unwrap();

    // Create vec0 table for 1024d
    brain_backend::db::ensure_vec_table(&conn, 1024).unwrap();

    conn
}

#[test]
fn test_fts5_basic_search() {
    let conn = setup_db();
    let repo = brain_backend::memory::MemoryRepository::new(&conn);

    // Insert memories with different content
    let mem1 = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: Some(1),
        run_id: None,
        content: "The quick brown fox jumps over the lazy dog".to_string(),
        content_hash: brain_backend::memory::compute_content_hash("The quick brown fox jumps over the lazy dog"),
        memory_type: "fact".to_string(),
        layer: "project".to_string(),
        importance: 0.8,
        source: "user".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };
    let mem2 = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: Some(1),
        run_id: None,
        content: "Rust is a systems programming language focused on safety".to_string(),
        content_hash: brain_backend::memory::compute_content_hash("Rust is a systems programming language focused on safety"),
        memory_type: "fact".to_string(),
        layer: "project".to_string(),
        importance: 0.9,
        source: "user".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };
    let mem3 = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: Some(1),
        run_id: None,
        content: "The quick brown fox appears in many pangrams".to_string(),
        content_hash: brain_backend::memory::compute_content_hash("The quick brown fox appears in many pangrams"),
        memory_type: "fact".to_string(),
        layer: "project".to_string(),
        importance: 0.6,
        source: "agent".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };

    let id1 = repo.insert(&mem1).unwrap();
    let id2 = repo.insert(&mem2).unwrap();
    let id3 = repo.insert(&mem3).unwrap();

    // FTS5 search for "fox"
    let retriever = brain_backend::memory::MemoryRetriever::new(&conn);
    let results = retriever.fts_search("fox", Some(1), 10).unwrap();

    assert!(!results.is_empty(), "FTS search for 'fox' should return results");
    let found_ids: Vec<i64> = results.iter().map(|(id, _)| *id).collect();
    assert!(found_ids.contains(&id1), "should find mem1 about fox");
    assert!(found_ids.contains(&id3), "should find mem3 about fox");
    assert!(!found_ids.contains(&id2), "should NOT find mem2 about Rust");

    // FTS search for "programming"
    let results2 = retriever.fts_search("programming", Some(1), 10).unwrap();
    let found_ids2: Vec<i64> = results2.iter().map(|(id, _)| *id).collect();
    assert!(found_ids2.contains(&id2), "should find mem2 about programming");
    assert_eq!(found_ids2.len(), 1, "only one result about programming");
}

#[test]
fn test_content_hash_dedup() {
    let conn = setup_db();
    let repo = brain_backend::memory::MemoryRepository::new(&conn);

    let content = "Deduplication test content for hashing";
    let mem = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: Some(1),
        run_id: None,
        content: content.to_string(),
        content_hash: brain_backend::memory::compute_content_hash(content),
        memory_type: "fact".to_string(),
        layer: "project".to_string(),
        importance: 0.5,
        source: "user".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };

    let id1 = repo.insert(&mem).unwrap();
    assert!(id1 > 0);

    // Duplicate should be caught by unique index
    let result = repo.insert(&mem);
    assert!(result.is_err(), "inserting duplicate should fail");

    // But find_active_by_hash works
    let hash = brain_backend::memory::compute_content_hash(content);
    let found = repo.find_active_by_hash(Some(1), 1, &hash).unwrap();
    assert_eq!(found, Some(id1));
}

#[test]
fn test_project_isolation() {
    let conn = setup_db();
    let repo = brain_backend::memory::MemoryRepository::new(&conn);

    // Insert memories for project 1 and project 2
    let mem_p1 = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: Some(1),
        run_id: None,
        content: "Memory belonging to project one with unique content A".to_string(),
        content_hash: brain_backend::memory::compute_content_hash("Memory belonging to project one with unique content A"),
        memory_type: "fact".to_string(),
        layer: "project".to_string(),
        importance: 0.7,
        source: "user".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };
    let mem_p2 = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: Some(2),
        run_id: None,
        content: "Memory belonging to project two with unique content B".to_string(),
        content_hash: brain_backend::memory::compute_content_hash("Memory belonging to project two with unique content B"),
        memory_type: "fact".to_string(),
        layer: "project".to_string(),
        importance: 0.7,
        source: "user".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };

    let _id1 = repo.insert(&mem_p1).unwrap();
    let _id2 = repo.insert(&mem_p2).unwrap();

    // FTS search scoped to project 1 should NOT return project 2 memory
    let retriever = brain_backend::memory::MemoryRetriever::new(&conn);
    let results_p1 = retriever.fts_search("Memory belonging", Some(1), 10).unwrap();
    for (id, _) in &results_p1 {
        let mem = repo.get_active(*id).unwrap().unwrap();
        assert_eq!(mem.project_id, Some(1), "project 1 search should only return project 1 memories");
    }

    // FTS search scoped to project 2
    let results_p2 = retriever.fts_search("Memory belonging", Some(2), 10).unwrap();
    for (id, _) in &results_p2 {
        let mem = repo.get_active(*id).unwrap().unwrap();
        assert_eq!(mem.project_id, Some(2), "project 2 search should only return project 2 memories");
    }

    // List by project
    let list_p1 = repo.list_by_project(1, None, 10).unwrap();
    assert_eq!(list_p1.len(), 1, "project 1 should have exactly 1 memory");
    assert_eq!(list_p1[0].project_id, Some(1));

    let list_p2 = repo.list_by_project(2, None, 10).unwrap();
    assert_eq!(list_p2.len(), 1, "project 2 should have exactly 1 memory");
    assert_eq!(list_p2[0].project_id, Some(2));
}

#[test]
fn test_memory_layers() {
    let conn = setup_db();
    let repo = brain_backend::memory::MemoryRepository::new(&conn);

    // global_profile (project_id = NULL)
    let gp = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: None,
        run_id: None,
        content: "Global user profile memory for system-wide preferences".to_string(),
        content_hash: brain_backend::memory::compute_content_hash("Global user profile memory for system-wide preferences"),
        memory_type: "fact".to_string(),
        layer: "global_profile".to_string(),
        importance: 0.95,
        source: "user".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };

    // project
    let proj = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: Some(1),
        run_id: None,
        content: "Project-specific memory about code architecture".to_string(),
        content_hash: brain_backend::memory::compute_content_hash("Project-specific memory about code architecture"),
        memory_type: "fact".to_string(),
        layer: "project".to_string(),
        importance: 0.8,
        source: "agent".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };

    // episodic
    let episodic = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: Some(1),
        run_id: Some(1),
        content: "Episodic memory from a specific run execution event".to_string(),
        content_hash: brain_backend::memory::compute_content_hash("Episodic memory from a specific run execution event"),
        memory_type: "episode".to_string(),
        layer: "episodic".to_string(),
        importance: 0.5,
        source: "system".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };

    // working
    let working = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: Some(1),
        run_id: Some(1),
        content: "Working memory for current task context and state".to_string(),
        content_hash: brain_backend::memory::compute_content_hash("Working memory for current task context and state"),
        memory_type: "fact".to_string(),
        layer: "working".to_string(),
        importance: 0.3,
        source: "system".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };

    let id_gp = repo.insert(&gp).unwrap();
    let id_proj = repo.insert(&proj).unwrap();
    let id_ep = repo.insert(&episodic).unwrap();
    let id_wk = repo.insert(&working).unwrap();

    // List by layer
    let gp_list = repo.list_by_layer("global_profile", None, 10).unwrap();
    assert_eq!(gp_list.len(), 1);
    assert_eq!(gp_list[0].id, id_gp);

    let proj_list = repo.list_by_layer("project", Some(1), 10).unwrap();
    assert_eq!(proj_list.len(), 1);
    assert_eq!(proj_list[0].id, id_proj);

    let ep_list = repo.list_by_layer("episodic", Some(1), 10).unwrap();
    assert_eq!(ep_list.len(), 1);
    assert_eq!(ep_list[0].id, id_ep);

    let wk_list = repo.list_by_layer("working", Some(1), 10).unwrap();
    assert_eq!(wk_list.len(), 1);
    assert_eq!(wk_list[0].id, id_wk);

    // global_profile list
    let gp_global = repo.list_global_profile(10).unwrap();
    assert_eq!(gp_global.len(), 1);
    assert_eq!(gp_global[0].id, id_gp);
}

#[test]
fn test_heuristic_filter() {
    use brain_backend::memory::heuristic;

    // Too short
    let r = heuristic::check_content("hi");
    assert!(!r.passed);
    assert!(r.reason.unwrap().contains("too short"));

    // Too few words
    let r = heuristic::check_content("hello world");
    assert!(!r.passed);
    assert!(r.reason.unwrap().contains("too few words"));

    // Junk pattern
    let r = heuristic::check_content("test test");
    assert!(!r.passed);

    // Only digits
    let r = heuristic::check_content("12345 67890 11111");
    assert!(!r.passed);

    // Good content
    let r = heuristic::check_content("This is a meaningful memory about Rust programming");
    assert!(r.passed);
    assert!(r.reason.is_none());
}

#[test]
fn test_heuristic_layer_validation() {
    use brain_backend::memory::heuristic;

    // global_profile must NOT have project_id
    assert!(heuristic::validate_layer_for_project("global_profile", None).is_ok());
    assert!(heuristic::validate_layer_for_project("global_profile", Some(1)).is_err());

    // project MUST have project_id
    assert!(heuristic::validate_layer_for_project("project", Some(1)).is_ok());
    assert!(heuristic::validate_layer_for_project("project", None).is_err());

    // episodic MUST have project_id
    assert!(heuristic::validate_layer_for_project("episodic", Some(1)).is_ok());
    assert!(heuristic::validate_layer_for_project("episodic", None).is_err());

    // working MUST have project_id
    assert!(heuristic::validate_layer_for_project("working", Some(1)).is_ok());
    assert!(heuristic::validate_layer_for_project("working", None).is_err());

    // invalid layer
    assert!(heuristic::validate_layer_for_project("invalid", Some(1)).is_err());
}

#[test]
fn test_supersede() {
    let conn = setup_db();
    let repo = brain_backend::memory::MemoryRepository::new(&conn);

    let old = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: Some(1),
        run_id: None,
        content: "Old version of a memory that will be superseded".to_string(),
        content_hash: brain_backend::memory::compute_content_hash("Old version of a memory that will be superseded"),
        memory_type: "fact".to_string(),
        layer: "project".to_string(),
        importance: 0.5,
        source: "user".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };

    let new = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: Some(1),
        run_id: None,
        content: "New version of a memory that supersedes the old one".to_string(),
        content_hash: brain_backend::memory::compute_content_hash("New version of a memory that supersedes the old one"),
        memory_type: "fact".to_string(),
        layer: "project".to_string(),
        importance: 0.7,
        source: "agent".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };

    let old_id = repo.insert(&old).unwrap();
    let new_id = repo.insert(&new).unwrap();

    // Old should be active
    let old_mem = repo.get_active(old_id).unwrap();
    assert!(old_mem.is_some());

    // Supersede
    repo.supersede(old_id, new_id).unwrap();

    // Old should no longer be active
    let old_mem = repo.get_active(old_id).unwrap();
    assert!(old_mem.is_none(), "superseded memory should not be active");

    // New should be active
    let new_mem = repo.get_active(new_id).unwrap();
    assert!(new_mem.is_some());
}

#[test]
fn test_touch_and_count() {
    let conn = setup_db();
    let repo = brain_backend::memory::MemoryRepository::new(&conn);

    let mem = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: Some(1),
        run_id: None,
        content: "Memory for touch and access count testing purposes".to_string(),
        content_hash: brain_backend::memory::compute_content_hash("Memory for touch and access count testing purposes"),
        memory_type: "fact".to_string(),
        layer: "project".to_string(),
        importance: 0.5,
        source: "user".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };

    let id = repo.insert(&mem).unwrap();

    // Initial count should be 0
    let m = repo.get_active(id).unwrap().unwrap();
    assert_eq!(m.access_count, 0);

    // Touch once
    repo.touch(id).unwrap();
    let m = repo.get_active(id).unwrap().unwrap();
    assert_eq!(m.access_count, 1);

    // Touch batch
    repo.touch_batch(&[id]).unwrap();
    let m = repo.get_active(id).unwrap().unwrap();
    assert_eq!(m.access_count, 2);

    // Count active
    let count = repo.count_active(Some(1)).unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_vec0_knn_search() {
    let conn = setup_db();
    let repo = brain_backend::memory::MemoryRepository::new(&conn);
    let store = brain_backend::memory::MemoryEmbeddingStore::new(&conn);

    // Insert memories with fake embeddings (1024d)
    let mut emb1 = vec![0.0f32; 1024];
    emb1[0] = 1.0;
    let mut emb2 = vec![0.0f32; 1024];
    emb2[1] = 1.0;

    let mem1 = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: Some(1),
        run_id: None,
        content: "First memory with unique content about alpha topic".to_string(),
        content_hash: brain_backend::memory::compute_content_hash("First memory with unique content about alpha topic"),
        memory_type: "fact".to_string(),
        layer: "project".to_string(),
        importance: 0.5,
        source: "user".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };
    let mem2 = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: Some(1),
        run_id: None,
        content: "Second memory with unique content about beta topic".to_string(),
        content_hash: brain_backend::memory::compute_content_hash("Second memory with unique content about beta topic"),
        memory_type: "fact".to_string(),
        layer: "project".to_string(),
        importance: 0.5,
        source: "user".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };

    let id1 = repo.insert_atomic(&mem1, Some(&emb1)).unwrap();
    let _id2 = repo.insert_atomic(&mem2, Some(&emb2)).unwrap();

    // Search with query close to emb1
    let results = store.search_knn(&emb1, 1024, 2, &[]).unwrap();
    assert!(!results.is_empty(), "KNN search should return results");

    // First result should be id1 (exact match, distance 0)
    assert_eq!(results[0].memory_id, id1, "first result should be the exact match");
    assert!(results[0].distance < 0.01, "exact match distance should be near 0");

    // Count
    let count = store.count(1024).unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_vec0_search_project_isolation() {
    let conn = setup_db();
    let repo = brain_backend::memory::MemoryRepository::new(&conn);
    let retriever = brain_backend::memory::MemoryRetriever::new(&conn);

    // Insert memories with embeddings for different projects
    let mut emb = vec![0.0f32; 1024];
    emb[0] = 1.0;

    let mem_p1 = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: Some(1),
        run_id: None,
        content: "Project one vector search isolation test memory content".to_string(),
        content_hash: brain_backend::memory::compute_content_hash("Project one vector search isolation test memory content"),
        memory_type: "fact".to_string(),
        layer: "project".to_string(),
        importance: 0.8,
        source: "user".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };
    let mem_p2 = brain_backend::memory::repository::NewMemory {
        collection_id: 1,
        project_id: Some(2),
        run_id: None,
        content: "Project two vector search isolation test memory content".to_string(),
        content_hash: brain_backend::memory::compute_content_hash("Project two vector search isolation test memory content"),
        memory_type: "fact".to_string(),
        layer: "project".to_string(),
        importance: 0.8,
        source: "user".to_string(),
        source_ref: None,
        metadata_json: "{}".to_string(),
    };

    let _id1 = repo.insert_atomic(&mem_p1, Some(&emb)).unwrap();
    let _id2 = repo.insert_atomic(&mem_p2, Some(&emb)).unwrap();

    // Vector search scoped to project 1 via retriever
    let result = retriever.retrieve("test query", Some(1), 1, Some(&emb), 10).unwrap();
    for mem in &result.memories {
        assert_eq!(mem.project_id, Some(1), "search scoped to project 1 should only return project 1 memories");
    }
}
