use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

/// Reducer assignment - which keys this reducer is responsible for
pub struct ReducerAssignment {
    pub keys: Vec<String>,
}

/// Reducer worker that sums up vectors into final counts
pub struct Reducer {
    id: usize,
    shared_map: Arc<Mutex<HashMap<String, Vec<i32>>>>,
    task_handle: Option<JoinHandle<()>>,
}

impl Reducer {
    pub fn new(id: usize, shared_map: Arc<Mutex<HashMap<String, Vec<i32>>>>) -> Self {
        Self {
            id,
            shared_map,
            task_handle: None,
        }
    }

    /// Starts reducing values for assigned keys
    pub fn start(&mut self, assignment: ReducerAssignment) {
        let id = self.id;
        let shared_map = self.shared_map.clone();

        let handle = tokio::spawn(async move {
            if id.is_multiple_of(2) {
                println!("Reducer {} started for {} keys", id, assignment.keys.len());
            }

            for key in assignment.keys {
                // Get the vector for this key and sum it
                let count = {
                    let map = shared_map.lock().unwrap();
                    if let Some(vec) = map.get(&key) {
                        vec.iter().sum::<i32>()
                    } else {
                        0
                    }
                };

                // Update the shared map with the final count
                // We replace Vec<i32> with the summed count by storing it as a single-element vec
                let mut map = shared_map.lock().unwrap();
                map.insert(key.clone(), vec![count]);
            }

            if id.is_multiple_of(2) {
                println!("Reducer {} finished", id);
            }
        });

        self.task_handle = Some(handle);
    }

    /// Waits for the reducer task to complete
    pub async fn wait(self) -> Result<(), tokio::task::JoinError> {
        if let Some(handle) = self.task_handle {
            handle.await
        } else {
            Ok(())
        }
    }
}
