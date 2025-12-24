use crate::work_distributor::WorkDistributor;

/// Orchestrator coordinates the map-reduce workflow
/// Generic over mapper and reducer distributors
pub struct Orchestrator<MD: WorkDistributor, RD: WorkDistributor> {
    mapper_distributor: MD,
    reducer_distributor: RD,
}

impl<MD: WorkDistributor, RD: WorkDistributor> Orchestrator<MD, RD> {
    pub fn new(mapper_distributor: MD, reducer_distributor: RD) -> Self {
        Self {
            mapper_distributor,
            reducer_distributor,
        }
    }

    /// Runs the complete map-reduce workflow
    /// Generic over assignment types - implementations provide concrete types
    #[allow(clippy::too_many_arguments)]
    pub async fn run<MA, RA>(
        mut self,
        mappers: Vec<MD::Worker>,
        reducers: Vec<RD::Worker>,
        create_mapper_assignments: impl FnOnce(Vec<String>, Vec<String>, usize) -> Vec<MA>,
        create_reducer_assignments: impl FnOnce(Vec<String>, usize) -> Vec<RA>,
        data: Vec<String>,
        targets: Vec<String>,
        partition_size: usize,
        keys_per_reducer: usize,
    ) where
        MD::Worker: crate::worker::Worker<Assignment = MA>,
        RD::Worker: crate::worker::Worker<Assignment = RA>,
        MA: Clone,
        RA: Clone,
    {
        println!("=== ORCHESTRATOR STARTED ===");

        // MAP PHASE - Distribute work to mappers
        println!("\n=== MAP PHASE ===");
        println!("Distributing data to {} mappers...", mappers.len());

        // Create mapper assignments using factory function
        let mapper_assignments = create_mapper_assignments(data, targets.clone(), partition_size);

        // Distribute work to mappers
        self.mapper_distributor
            .distribute(mappers, mapper_assignments)
            .await;
        println!("All mappers completed!");

        // REDUCE PHASE - Assign work to reducers
        println!("\n=== REDUCE PHASE ===");
        println!("Starting {} reducers...", reducers.len());

        // Partition the keys among reducers based on keys_per_reducer
        let num_key_partitions = targets.len().div_ceil(keys_per_reducer);
        println!(
            "Distributing {} key partitions to {} reducers...",
            num_key_partitions,
            reducers.len()
        );

        // Create reducer assignments using factory function
        let reducer_assignments = create_reducer_assignments(targets, keys_per_reducer);

        // Distribute work to reducers
        self.reducer_distributor
            .distribute(reducers, reducer_assignments)
            .await;
        println!("All reducers completed!");

        println!("\n=== ORCHESTRATOR FINISHED ===");
    }
}
