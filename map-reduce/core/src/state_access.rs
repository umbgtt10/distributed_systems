/// Trait for accessing shared state across workers
/// Abstracts the storage mechanism (local, Redis, RPC, etc.)
pub trait StateAccess: Clone + Send + Sync + 'static {
    /// Initialize keys with empty vectors
    fn initialize(&self, keys: Vec<String>);

    /// Update a key with a value (append for mappers)
    fn update(&self, key: String, value: i32);

    /// Replace the entire value for a key (used by reducers)
    fn replace(&self, key: String, value: i32);

    /// Get all values for a key
    fn get(&self, key: &str) -> Vec<i32>;
}
