// Copyright 2025 Umberto Gotti <umberto.gotti@umbertogotti.dev>
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use async_trait::async_trait;
use key_value_server_core::{Storage, StorageError};
use sled::Db;
use std::{collections::HashMap, sync::Arc};
use tokio::task::spawn_blocking;

#[derive(Clone)]
pub struct SledDbStorage {
    db: Arc<Db>,
}

impl SledDbStorage {
    pub fn new(file_path: String) -> Self {
        Self {
            db: Arc::new(sled::open(file_path).unwrap()),
        }
    }
}

#[async_trait]
impl Storage for SledDbStorage {
    async fn get(&self, key: &str) -> Result<(String, u64), StorageError> {
        let key = key.to_string();
        let db = self.db.clone();
        spawn_blocking(move || {
            let key_bytes = key.as_bytes();
            let value_bytes = db
                .get(key_bytes)
                .map_err(|e| StorageError::StorageError(e.to_string()))?;
            if let Some(value_bytes) = value_bytes {
                let (value, version) = serde_json::from_slice(&value_bytes)
                    .map_err(|e| StorageError::StorageError(e.to_string()))?;
                Ok((value, version))
            } else {
                Err(StorageError::KeyNotFound(key))
            }
        })
        .await
        .map_err(|e| StorageError::StorageError(e.to_string()))?
    }

    async fn put(
        &self,
        key: &str,
        value: String,
        expected_version: u64,
    ) -> Result<u64, StorageError> {
        let key = key.to_string();
        let db = self.db.clone();
        spawn_blocking(move || {
            let key_bytes = key.as_bytes();
            let value_bytes = db
                .get(key_bytes)
                .map_err(|e| StorageError::StorageError(e.to_string()))?;

            if expected_version == 0 {
                // Check if key already exists and is valid
                if let Some(ref vb) = value_bytes {
                    if serde_json::from_slice::<(String, u64)>(vb).is_ok() {
                        return Err(StorageError::KeyAlreadyExists(key.to_string()));
                    }
                    // If corrupted, allow overwrite
                }

                let new_value_bytes = serde_json::to_vec(&(value.clone(), 1u64))
                    .map_err(|e| StorageError::StorageError(e.to_string()))?;
                db.insert(key_bytes, new_value_bytes)
                    .map_err(|e| StorageError::StorageError(e.to_string()))?;
                db.flush()
                    .map_err(|e| StorageError::StorageError(e.to_string()))?;

                Ok(1)
            } else {
                // Get current value and version
                match value_bytes {
                    Some(value_bytes) => {
                        let (_, current_version): (String, u64) =
                            serde_json::from_slice(&value_bytes)
                                .map_err(|e| StorageError::StorageError(e.to_string()))?;
                        if current_version == expected_version {
                            let new_version = expected_version + 1;
                            let new_value_bytes = serde_json::to_vec(&(value.clone(), new_version))
                                .map_err(|e| StorageError::StorageError(e.to_string()))?;
                            db.insert(key_bytes, new_value_bytes)
                                .map_err(|e| StorageError::StorageError(e.to_string()))?;
                            db.flush()
                                .map_err(|e| StorageError::StorageError(e.to_string()))?;

                            Ok(new_version)
                        } else {
                            Err(StorageError::VersionMismatch {
                                expected: expected_version,
                                actual: current_version,
                            })
                        }
                    }
                    None => Err(StorageError::KeyNotFound(key.to_string())),
                }
            }
        })
        .await
        .map_err(|e| StorageError::StorageError(format!("Task panicked: {:?}", e)))?
    }

    async fn print_all(&self) {
        let db = self.db.clone();
        let data: HashMap<String, (String, u64)> = spawn_blocking(move || {
            let mut map = HashMap::new();
            for result in db.iter() {
                let (key_bytes, value_bytes) = match result {
                    Ok(pair) => pair,
                    Err(e) => {
                        eprintln!("Database iter error: {}", e);
                        continue;
                    }
                };
                let key = match String::from_utf8(key_bytes.to_vec()) {
                    Ok(k) => k,
                    Err(e) => {
                        eprintln!("Invalid UTF-8 key: {}", e);
                        continue;
                    }
                };
                let (value, version): (String, u64) = match serde_json::from_slice(&value_bytes) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("Deserialization error for key {}: {}", key, e);
                        ("deserialization_error".to_string(), 0)
                    }
                };
                map.insert(key, (value, version));
            }
            map
        })
        .await
        .unwrap_or_else(|e| {
            eprintln!("Task panicked in print_all: {:?}", e);
            HashMap::new()
        });

        println!("\n=== Final Storage State ===");
        if data.is_empty() {
            println!("  No keys in storage");
        } else {
            let mut keys: Vec<_> = data.keys().cloned().collect();
            keys.sort();
            for key in keys {
                if let Some((value, version)) = data.get(&key) {
                    println!("  '{}' -> value='{}', version={}", key, value, version);
                }
            }
        }
        println!("===========================\n");
    }
}
