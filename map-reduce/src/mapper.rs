use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Work assignment for a mapper - describes what chunk to process
#[derive(Clone)]
pub struct WorkAssignment {
    pub chunk_id: usize,
    pub data: Vec<String>,
    pub targets: Vec<String>,
}

/// Mapper worker that searches for target words in its data chunk
pub struct Mapper {
    id: usize,
    shared_map: Arc<Mutex<HashMap<String, Vec<i32>>>>,
    cancel_token: CancellationToken,
    task_handle: Option<JoinHandle<()>>,
}

impl Mapper {
    pub fn new(
        id: usize,
        shared_map: Arc<Mutex<HashMap<String, Vec<i32>>>>,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            id,
            shared_map,
            cancel_token,
            task_handle: None,
        }
    }

    /// Starts processing the assigned data chunk
    pub fn start(&mut self, assignment: WorkAssignment) {
        let id = self.id;
        let shared_map = self.shared_map.clone();
        let cancel_token = self.cancel_token.clone();

        let handle = tokio::spawn(async move {
            if id.is_multiple_of(10) {
                println!(
                    "Mapper {} processing {} items from chunk {}",
                    id,
                    assignment.data.len(),
                    assignment.chunk_id
                );
            }

            // Process each string in the chunk
            for text in assignment.data {
                // Check for cancellation
                if cancel_token.is_cancelled() {
                    println!("Mapper {} cancelled", id);
                    return;
                }

                // Search for each target word in the text
                for target in &assignment.targets {
                    if text.contains(target.as_str()) {
                        // Found a match! Add 1 to the vector for this target
                        let mut map = shared_map.lock().unwrap();
                        if let Some(vec) = map.get_mut(target) {
                            vec.push(1);
                        }
                    }
                }
            }

            if id.is_multiple_of(10) {
                println!("Mapper {} finished chunk {}", id, assignment.chunk_id);
            }
        });

        self.task_handle = Some(handle);
    }

    /// Waits for the mapper task to complete
    pub async fn wait(self) -> Result<(), tokio::task::JoinError> {
        if let Some(handle) = self.task_handle {
            handle.await
        } else {
            Ok(())
        }
    }
}
