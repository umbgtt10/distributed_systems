// Copyright 2025 Umberto Gotti <umberto.gotti@umbertogotti.dev>
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

use key_value_server_core::{Storage, StorageError};
use std::{collections::HashMap, path::Path, sync::Arc};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    sync::Mutex,
};

#[derive(Clone)]
pub struct FlatFileStorage {
    file_path: String,
    mutex: Arc<Mutex<()>>,
}

impl FlatFileStorage {
    pub async fn new(file_path: String) -> Self {
        if !Path::new(&file_path).exists() {
            File::create(&file_path)
                .await
                .expect("Failed to create file");
        }

        Self {
            file_path,
            mutex: Arc::new(Mutex::new(())),
        }
    }

    async fn get(&self, key: &str) -> Option<(String, u64)> {
        let file = File::open(&self.file_path).await.ok()?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let parts: Vec<&str> = line.split(',').collect();

            if parts.len() != 3 {
                eprintln!("Skipping malformed line while reading: {}", line);
                continue;
            }
            let stored_key = parts[0];
            let stored_value = parts[1];
            let stored_version: u64 = parts[2].parse().unwrap_or(0);

            if stored_key == key {
                return Some((stored_value.to_string(), stored_version));
            }
        }

        None
    }
}

#[async_trait::async_trait]
impl Storage for FlatFileStorage {
    async fn get(&self, key: &str) -> Result<(String, u64), StorageError> {
        let _lock = self.mutex.lock().await;
        let entry = self.get(key).await;
        if let Some((value, version)) = entry {
            return Ok((value, version));
        }

        Err(StorageError::KeyNotFound(key.to_string()))
    }

    async fn put(
        &self,
        key: &str,
        value: String,
        expected_version: u64,
    ) -> Result<u64, StorageError> {
        let _lock = self.mutex.lock().await;
        let entry = self.get(key).await;
        if expected_version == 0 {
            if entry.is_some() {
                return Err(StorageError::KeyAlreadyExists(key.to_string()));
            }

            let file = OpenOptions::new()
                .append(true)
                .open(&self.file_path)
                .await
                .expect("Failed to open file for append");

            let mut writer = BufWriter::new(file);
            let line = format!("{},{},1\n", key, value);
            writer
                .write_all(line.as_bytes())
                .await
                .expect("Failed to write");
            writer.flush().await.expect("Failed to flush");

            Ok(1)
        } else {
            match entry {
                Some((_, current_version)) => {
                    if current_version == expected_version {
                        let new_version = expected_version + 1;

                        // Rewrite the entire file with the updated value
                        let mut lines = Vec::new();
                        let file = File::open(&self.file_path)
                            .await
                            .expect("Failed to open file for read");
                        let reader = BufReader::new(file);
                        let mut line_iter = reader.lines();
                        while let Ok(Some(line)) = line_iter.next_line().await {
                            let parts: Vec<&str> = line.split(',').collect();
                            if parts.len() != 3 {
                                eprintln!("Skipping malformed line during update: {}", line);
                                continue;
                            }
                            let stored_key = parts[0];
                            if stored_key == key {
                                lines.push(format!("{},{},{}", key, value, new_version));
                            } else {
                                lines.push(line);
                            }
                        }

                        // Truncate and rewrite the file
                        let file = OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .open(&self.file_path)
                            .await
                            .expect("Failed to open file for write");
                        file.set_len(0).await.expect("Failed to truncate file");
                        let mut writer = BufWriter::new(file);
                        for line in lines {
                            writer
                                .write_all(line.as_bytes())
                                .await
                                .expect("Failed to write line");
                            writer
                                .write_all(b"\n")
                                .await
                                .expect("Failed to write newline");
                        }
                        writer.flush().await.expect("Failed to flush writer");

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
    }

    async fn print_all(&self) {
        let _lock = self.mutex.lock().await;
        let file = File::open(&self.file_path)
            .await
            .expect("Failed to open file for read");
        let mut data = HashMap::new();
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() != 3 {
                eprintln!("Skipping malformed line while printing: {}", line);
                continue;
            }
            let stored_key = parts[0].to_string();
            let stored_value = parts[1].to_string();
            let stored_version: u64 = parts[2].parse().unwrap_or(0);

            data.insert(stored_key, (stored_value, stored_version));
        }

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
