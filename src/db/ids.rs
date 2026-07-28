#![allow(dead_code, unused_imports)] // SCAFFOLD — temporary until all modules consume IDs

use uuid::Uuid;
use rusqlite::types::Value;

pub fn new_uuid() -> Uuid {
    Uuid::now_v7()
}

pub fn new_uuid_bytes() -> [u8; 16] {
    *new_uuid().as_bytes()
}

pub fn new_uuid_blob() -> Vec<u8> {
    new_uuid_bytes().to_vec()
}

pub fn uuid_to_value(uuid: &Uuid) -> Value {
    Value::Blob(uuid.as_bytes().to_vec())
}

pub fn bytes_to_uuid(bytes: &[u8]) -> anyhow::Result<Uuid> {
    Uuid::from_slice(bytes).map_err(|e| anyhow::anyhow!("invalid UUID bytes: {}", e))
}
