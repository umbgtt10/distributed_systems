mod mapper;
mod orchestrator;
mod reducer;

use orchestrator::Orchestrator;
use rand::Rng;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Deserialize)]
struct Config {
    num_strings: usize,
    max_string_length: usize,
    num_target_words: usize,
    target_word_length: usize,
    partition_size: usize,
    num_mappers: usize,
    num_reducers: usize,
}

impl Config {
    fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&contents)?;
        Ok(config)
    }
}

/// Generate a random string of up to max_len characters
fn generate_random_string(rng: &mut impl Rng, max_len: usize) -> String {
    let len = rng.gen_range(1..=max_len);
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..26);
            (b'a' + idx) as char
        })
        .collect()
}

/// Generate a random 3-digit word (3 characters)
fn generate_target_word(rng: &mut impl Rng, length: usize) -> String {
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..26);
            (b'a' + idx) as char
        })
        .collect()
}

#[tokio::main]
async fn main() {
    let start_time = Instant::now();

    // Load configuration from JSON file
    let config = match Config::load("config.json") {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load config.json: {}", e);
            eprintln!("Using default configuration...");
            Config {
                num_strings: 1_000_000,
                max_string_length: 20,
                num_target_words: 100,
                target_word_length: 3,
                partition_size: 10_000,
                num_mappers: 100,
                num_reducers: 10,
            }
        }
    };

    println!("=== MAP-REDUCE WORD SEARCH ===");
    println!("Configuration:");
    println!("  - Strings: {}", config.num_strings);
    println!("  - Max string length: {}", config.max_string_length);
    println!("  - Target words: {}", config.num_target_words);
    println!("  - Target word length: {}", config.target_word_length);
    println!("  - Partition size: {}", config.partition_size);
    println!("  - Mappers: {}", config.num_mappers);
    println!("  - Reducers: {}", config.num_reducers);
    println!("\nGenerating data...");

    let mut rng = rand::thread_rng();

    // Generate random strings
    let data: Vec<String> = (0..config.num_strings)
        .map(|_| generate_random_string(&mut rng, config.max_string_length))
        .collect();

    println!("Generated {} strings", data.len());

    // Generate random target words
    let targets: Vec<String> = (0..config.num_target_words)
        .map(|_| generate_target_word(&mut rng, config.target_word_length))
        .collect();

    println!("Generated {} target words", targets.len());

    // Create shared HashMap<String, Vec<i32>> for mappers to update
    let shared_map: Arc<Mutex<HashMap<String, Vec<i32>>>> = Arc::new(Mutex::new(
        targets
            .iter()
            .map(|word| (word.clone(), Vec::new()))
            .collect(),
    ));

    // Partition data into chunks based on partition_size
    let mut data_chunks = Vec::new();
    let num_partitions = (data.len() + config.partition_size - 1) / config.partition_size;

    for i in 0..num_partitions {
        let start = i * config.partition_size;
        let end = std::cmp::min(start + config.partition_size, data.len());
        data_chunks.push(data[start..end].to_vec());
    }

    println!(
        "Partitioned data into {} chunks for {} mappers",
        data_chunks.len(),
        config.num_mappers
    );
    println!("\nStarting MapReduce...");

    // Create orchestrator and run
    let mut orchestrator = Orchestrator::new(config.num_mappers, config.num_reducers);
    let cancel_token = orchestrator.cancellation_token();

    // Setup Ctrl+C handler
    let ctrl_c_token = cancel_token.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
        println!("\n\n=== Ctrl+C received, initiating shutdown ===");
        ctrl_c_token.cancel();
    });

    // Run the orchestrator
    orchestrator
        .run(data_chunks, targets, shared_map.clone())
        .await;

    // Extract final results
    let final_results = shared_map.lock().unwrap();

    // Display results
    println!("\n=== RESULTS ===");
    let mut sorted_results: Vec<_> = final_results.iter().collect();
    sorted_results.sort_by(|a, b| {
        let a_count = a.1.first().unwrap_or(&0);
        let b_count = b.1.first().unwrap_or(&0);
        b_count.cmp(a_count).then(a.0.cmp(b.0))
    });

    let mut total_occurrences = 0;
    for (word, count_vec) in sorted_results.iter().take(20) {
        let count = count_vec.first().unwrap_or(&0);
        println!("{}: {}", word, count);
        total_occurrences += count;
    }

    if sorted_results.len() > 20 {
        println!("... ({} more words)", sorted_results.len() - 20);
        for (_, count_vec) in sorted_results.iter().skip(20) {
            let count = count_vec.first().unwrap_or(&0);
            total_occurrences += count;
        }
    }

    println!("\nTotal occurrences found: {}", total_occurrences);

    let elapsed = start_time.elapsed();
    println!("\n=== PROGRAM COMPLETE ===");
    println!("Total time: {:.2}s", elapsed.as_secs_f64());
}
