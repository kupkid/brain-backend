use rusqlite::{Connection, OptionalExtension, params};
use tracing::info;

use crate::db::ids;

#[derive(Debug, Clone)]
pub struct NewProject {
    pub name: String,
    pub config_json: String,
}

#[derive(Debug, Clone)]
pub struct StoredProject {
    pub id: i64,
    pub uuid: Vec<u8>,
    pub name: String,
    pub root_path: String,
    pub config_json: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct ProjectRepository<'a> {
    conn: &'a Connection,
    data_dir: std::path::PathBuf,
}

impl<'a> ProjectRepository<'a> {
    pub fn new(conn: &'a Connection, data_dir: std::path::PathBuf) -> Self {
        Self { conn, data_dir }
    }

    /// Create project with deterministic root_path
    pub fn create(&self, project: &NewProject) -> anyhow::Result<i64> {
        let uuid = ids::new_uuid();
        let uuid_hex = uuid.as_simple().to_string();
        let root_path = self
            .data_dir
            .join("projects")
            .join(&uuid_hex)
            .join("workspace");

        // Create workspace directory
        std::fs::create_dir_all(&root_path)?;

        let uuid_bytes = uuid.as_bytes().to_vec();

        self.conn.execute(
            "INSERT INTO projects (uuid, name, root_path, config_json)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                uuid_bytes,
                project.name,
                root_path.to_str().unwrap(),
                project.config_json
            ],
        )?;

        let id = self.conn.last_insert_rowid();
        info!(
            "created project: id={}, name={}, uuid={}",
            id, project.name, uuid_hex
        );
        Ok(id)
    }

    /// Get project by id
    pub fn get(&self, id: i64) -> anyhow::Result<Option<StoredProject>> {
        self.conn
            .query_row(
                "SELECT id, uuid, name, root_path, config_json, created_at, updated_at
                 FROM projects WHERE id = ?1",
                params![id],
                |r| {
                    Ok(StoredProject {
                        id: r.get(0)?,
                        uuid: r.get(1)?,
                        name: r.get(2)?,
                        root_path: r.get(3)?,
                        config_json: r.get(4)?,
                        created_at: r.get(5)?,
                        updated_at: r.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Get project by uuid
    pub fn get_by_uuid(&self, uuid: &[u8]) -> anyhow::Result<Option<StoredProject>> {
        self.conn
            .query_row(
                "SELECT id, uuid, name, root_path, config_json, created_at, updated_at
                 FROM projects WHERE uuid = ?1",
                params![uuid],
                |r| {
                    Ok(StoredProject {
                        id: r.get(0)?,
                        uuid: r.get(1)?,
                        name: r.get(2)?,
                        root_path: r.get(3)?,
                        config_json: r.get(4)?,
                        created_at: r.get(5)?,
                        updated_at: r.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// List all projects
    pub fn list(&self, limit: usize) -> anyhow::Result<Vec<StoredProject>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, uuid, name, root_path, config_json, created_at, updated_at
             FROM projects ORDER BY created_at DESC LIMIT ?1",
        )?;
        let projects = stmt
            .query_map(params![limit], |r| {
                Ok(StoredProject {
                    id: r.get(0)?,
                    uuid: r.get(1)?,
                    name: r.get(2)?,
                    root_path: r.get(3)?,
                    config_json: r.get(4)?,
                    created_at: r.get(5)?,
                    updated_at: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(projects)
    }

    /// Delete project
    pub fn delete(&self, id: i64) -> anyhow::Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }
}
