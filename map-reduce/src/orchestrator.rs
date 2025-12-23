use crate::mapper::{Mapper, WorkAssignment};
use crate::reducer::{Reducer, ReducerAssignment};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Orchestrator coordinates the map-reduce workflow
pub struct Orchestrator {
    cancellation_token: CancellationToken,
    num_mappers: usize,
    num_reducers: usize,
}

impl Orchestrator {
    pub fn new(num_mappers: usize, num_reducers: usize) -> Self {
        Self {
            cancellation_token: CancellationToken::new(),
            num_mappers,
            num_reducers,
        }
    }

    /// Returns a clone of the cancellation token for external control
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    /// Runs the complete map-reduce workflow
    pub async fn run(
        &mut self,
        data_chunks: Vec<Vec<String>>,
        targets: Vec<String>,
        shared_map: Arc<Mutex<HashMap<String, Vec<i32>>>>,
    ) {
        println!("=== ORCHESTRATOR STARTED ===");

        // MAP PHASE - Distribute work to mappers
        println!("\n=== MAP PHASE ===");
        println!(
            "Distributing {} chunks to {} mappers...",
            data_chunks.len(),
            self.num_mappers
        );

        // Create completion channel
        let (complete_tx, mut complete_rx) = mpsc::channel::<usize>(self.num_mappers);

        // Create mapper pool
        let mut mappers: Vec<Mapper> = Vec::new();
        for mapper_id in 0..self.num_mappers {
            let mapper = Mapper::new(
                mapper_id,
                shared_map.clone(),
                self.cancellation_token.clone(),
            );
            mappers.push(mapper);
        }

        // Track which chunks have been assigned and which mappers are available
        let mut chunk_index = 0;
        let mut active_mappers = 0;

        // Assign initial work to all mappers
        for mapper in mappers.iter_mut() {
            if chunk_index < data_chunks.len() {
                let assignment = WorkAssignment {
                    chunk_id: chunk_index,
                    data: data_chunks[chunk_index].clone(),
                    targets: targets.clone(),
                };
                let tx = complete_tx.clone();
                mapper.process_chunk(assignment, tx);
                chunk_index += 1;
                active_mappers += 1;
            }
        }

        // As mappers complete, assign them more work
        while active_mappers > 0 {
            if let Some(mapper_id) = complete_rx.recv().await {
                active_mappers -= 1;

                // Assign next chunk if available
                if chunk_index < data_chunks.len() {
                    let assignment = WorkAssignment {
                        chunk_id: chunk_index,
                        data: data_chunks[chunk_index].clone(),
                        targets: targets.clone(),
                    };
                    let tx = complete_tx.clone();
                    mappers[mapper_id].process_chunk(assignment, tx);
                    chunk_index += 1;
                    active_mappers += 1;
                }
            }
        }

        // Wait for all mappers to fully shut down
        println!("Waiting for all mappers to complete...");
        for (idx, mapper) in mappers.into_iter().enumerate() {
            if let Err(e) = mapper.wait().await {
                eprintln!("Mapper {} task failed: {}", idx, e);
            }
        }
        println!("All mappers completed!");

        // REDUCE PHASE - Assign work to reducers
        println!("\n=== REDUCE PHASE ===");
        println!("Starting {} reducers...", self.num_reducers);

        let keys_per_reducer = targets.len() / self.num_reducers;
        let mut reducers: Vec<Reducer> = Vec::new();

        // Partition the keys among reducers
        for reducer_id in 0..self.num_reducers {
            let start = reducer_id * keys_per_reducer;
            let end = if reducer_id == self.num_reducers - 1 {
                targets.len()
            } else {
                (reducer_id + 1) * keys_per_reducer
            };

            let assigned_keys = targets[start..end].to_vec();
            let assignment = ReducerAssignment {
                keys: assigned_keys,
            };

            let mut reducer = Reducer::new(reducer_id, shared_map.clone());
            reducer.start(assignment);
            reducers.push(reducer);
        }

        // Wait for all reducers to complete
        println!("Waiting for all reducers to complete...");
        for (idx, reducer) in reducers.into_iter().enumerate() {
            if let Err(e) = reducer.wait().await {
                eprintln!("Reducer {} task failed: {}", idx, e);
            }
        }
        println!("All reducers completed!");

        println!("\n=== ORCHESTRATOR FINISHED ===");
    }
}
