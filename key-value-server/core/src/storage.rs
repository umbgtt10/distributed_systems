/// Trait for abstracting key-value storage with versioning
/// Different implementations handle concurrency internally
pub trait Storage: Send + Sync {
    /// Get a value and its current version
    /// Returns error if the key doesn't exist
    fn get(&self, key: &str) -> Result<(String, u64), StorageError>;

    /// Put a value with optimistic concurrency control
    ///
    /// # Arguments
    /// * `key` - The key to store
    /// * `value` - The value to store
    /// * `expected_version` - Expected current version (0 = create new key)
    ///
    /// # Returns
    /// * `Ok(new_version)` - The new version after successful write
    /// * `Err(StorageError)` - Error if version mismatch, key exists/not found, etc.
    fn put(&self, key: &str, value: String, expected_version: u64) -> Result<u64, StorageError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// Key was not found (Get on non-existent key, or Put with version > 0 on non-existent key)
    KeyNotFound(String),

    /// Key already exists (Put with version = 0 on existing key)
    KeyAlreadyExists(String),

    /// Version mismatch (Put with wrong expected version)
    VersionMismatch { expected: u64, actual: u64 },

    /// Internal storage error (I/O, corruption, etc.)
    Internal(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::KeyNotFound(key) => write!(f, "Key '{}' not found", key),
            StorageError::KeyAlreadyExists(key) => write!(f, "Key '{}' already exists", key),
            StorageError::VersionMismatch { expected, actual } => {
                write!(
                    f,
                    "Version mismatch: expected {}, actual {}",
                    expected, actual
                )
            }
            StorageError::Internal(msg) => write!(f, "Internal storage error: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {}
