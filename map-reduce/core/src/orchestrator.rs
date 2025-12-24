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
    /// Generic over assignment types and context - implementations provide concrete types
    #[allow(clippy::too_many_arguments)]
    pub async fn run<MA, RA, I, C>(
        mut self,
        mappers: Vec<MD::Worker>,
        reducers: Vec<RD::Worker>,
        create_mapper_assignments: impl FnOnce(I, C, usize) -> Vec<MA>,
        create_reducer_assignments: impl FnOnce(C, usize) -> Vec<RA>,
        data: I,
        context: C,
        partition_size: usize,
        keys_per_reducer: usize,
    ) where
        MD::Worker: crate::worker::Worker<Assignment = MA>,
        RD::Worker: crate::worker::Worker<Assignment = RA>,
        MA: Clone,
        RA: Clone,
        C: Clone,
    {
        println!("=== ORCHESTRATOR STARTED ===");

        // MAP PHASE - Distribute work to mappers
        println!("\n=== MAP PHASE ===");
        println!("Distributing data to {} mappers...", mappers.len());

        // Create mapper assignments using factory function
        let mapper_assignments = create_mapper_assignments(data, context.clone(), partition_size);

        // Distribute work to mappers
        self.mapper_distributor
            .distribute(mappers, mapper_assignments)
            .await;
        println!("All mappers completed!");

        // REDUCE PHASE - Assign work to reducers
        println!("\n=== REDUCE PHASE ===");
        println!("Starting {} reducers...", reducers.len());

        // Create reducer assignments using factory function
        let reducer_assignments = create_reducer_assignments(context, keys_per_reducer);

        // Distribute work to reducers
        self.reducer_distributor
            .distribute(reducers, reducer_assignments)
            .await;
        println!("All reducers completed!");

        println!("\n=== ORCHESTRATOR FINISHED ===");
    }
}
