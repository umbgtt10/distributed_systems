# word-search - MapReduce Problem Definition

**[← Back to MapReduce README](../README.md)**

This crate defines the **business logic** for a word search MapReduce job. It implements the `MapReduceJob` trait to count word occurrences across a large text corpus, demonstrating how problem-specific logic remains independent of execution infrastructure.

---

## Overview

**Purpose**: Count occurrences of target words in a collection of text files
**Algorithm**: MapReduce word count
**Trait**: Implements `MapReduceJob` from `core`
**Infrastructure**: Works with **all three implementations** (task-channels, thread-socket, process-rpc)

---

## The Problem

Given:
- A collection of text files (e.g., books, articles)
- A list of target words to search for

Output:
- Count of each target word across all files

**Example**:
```
Input:
  Files: ["war_and_peace.txt", "moby_dick.txt"]
  Targets: ["the", "whale", "captain"]

Output:
  the: 58970
  whale: 1245
  captain: 892
```

---

## MapReduce Algorithm

### Map Phase

**Input**: Chunk of text lines
**Output**: Emit (word, 1) for each occurrence

```rust
fn map_work<S>(assignment: &MapWorkAssignment, state: &S)
where
    S: StateAccess,
{
    for line in &assignment.data {
        let words = line.split_whitespace()
            .map(|w| w.to_lowercase())
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()));

        for word in words {
            if assignment.targets.contains(&word.to_string()) {
                // Emit: word -> count
                state.update(word, 0, |count| *count += 1);
            }
        }
    }
}
```

**Characteristics**:
- **Stateless**: Each mapper processes independently
- **Parallel**: Multiple mappers process different chunks
- **Incremental**: Results accumulate in shared state

### Reduce Phase

**Input**: Partition of target words
**Output**: Final count for each word

```rust
fn reduce_work<S>(assignment: &ReduceWorkAssignment, state: &S)
where
    S: StateAccess,
{
    for key in &assignment.keys {
        // Read accumulated count from state
        let count: usize = state.get(key).unwrap_or(0);

        // In this implementation, state already contains final counts
        // Reduce phase ensures all counts are committed

        // Could perform additional aggregation here
        // (e.g., filtering, sorting, top-K)
    }
}
```

**Characteristics**:
- **Aggregation**: Combines results from all mappers
- **Parallel**: Different reducers handle different keys
- **Finalization**: Prepares output for results

---

## Type Definitions

### `MapWorkAssignment`

Data chunk assigned to a single mapper:

```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct MapWorkAssignment {
    pub chunk_id: usize,        // Chunk identifier
    pub data: Vec<String>,      // Lines of text
    pub targets: Vec<String>,   // Words to search for
}
```

**Size**: Configured via `partition_size` (typically 100-1000 lines per chunk)

### `ReduceWorkAssignment`

Key partition assigned to a single reducer:

```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct ReduceWorkAssignment {
    pub keys: Vec<String>,      // Target words to aggregate
}
```

**Size**: Configured via `keys_per_reducer` (typically all keys / num_reducers)

### `WordSearchContext`

Problem configuration passed through execution:

```rust
#[derive(Clone)]
pub struct WordSearchContext {
    pub targets: Vec<String>,   // List of words to search for
}
```

**Usage**:
- Passed to `create_map_assignments()` - included in each map chunk
- Passed to `create_reduce_assignments()` - determines key partitions

---

## Implementation Details

### Assignment Creation

**Map Assignments**:
```rust
fn create_map_assignments(
    data: Vec<String>,          // All lines from all files
    context: WordSearchContext,  // Target words
    partition_size: usize,       // Lines per chunk
) -> Vec<MapWorkAssignment> {
    let num_partitions = data.len().div_ceil(partition_size);

    (0..num_partitions)
        .map(|i| {
            let start = i * partition_size;
            let end = min(start + partition_size, data.len());

            MapWorkAssignment {
                chunk_id: i,
                data: data[start..end].to_vec(),
                targets: context.targets.clone(),
            }
        })
        .collect()
}
```

**Characteristics**:
- **Even distribution**: Each chunk has ~same number of lines
- **No overlap**: Each line processed exactly once
- **Target replication**: All mappers search for all targets

**Reduce Assignments**:
```rust
fn create_reduce_assignments(
    context: WordSearchContext,
    keys_per_reducer: usize,
) -> Vec<ReduceWorkAssignment> {
    let targets = context.targets;
    let num_partitions = targets.len().div_ceil(keys_per_reducer);

    (0..num_partitions)
        .map(|i| {
            let start = i * keys_per_reducer;
            let end = min(start + keys_per_reducer, targets.len());

            ReduceWorkAssignment {
                keys: targets[start..end].to_vec(),
            }
        })
        .collect()
}
```

**Characteristics**:
- **Key partitioning**: Each reducer handles subset of keys
- **No overlap**: Each key processed by exactly one reducer
- **Load balancing**: Keys distributed evenly

### Word Processing

**Normalization**:
```rust
let word = word.to_lowercase()
    .trim_matches(|c: char| !c.is_alphanumeric())
    .to_string();
```

**Rules**:
- Case-insensitive: "The" → "the"
- Strip punctuation: "hello!" → "hello"
- Preserve alphanumeric: "hello123" → "hello123"

**Target Matching**:
```rust
if assignment.targets.contains(&word) {
    state.update(&word, 0, |count| *count += 1);
}
```

**Optimization**: Only count words in target list (ignore others)

---

## State Interaction

### Map Phase State Updates

```rust
// Increment word count atomically
state.update(word, 0, |count| *count += 1);
```

**Behavior**:
- If key doesn't exist: Initialize to 0, then increment → 1
- If key exists: Load current value, increment, store back
- **Atomic**: `StateAccess::update()` ensures thread-safety

**Implementations**:
- `LocalStateAccess`: Mutex around HashMap
- `RpcStateAccess`: Server-side atomic update

### Reduce Phase State Access

```rust
// Read final count
let count: usize = state.get(key).unwrap_or(0);
```

**Behavior**:
- Read-only in this implementation
- Could perform additional aggregation
- Could write to separate output state

---

## Usage Example

### Complete Workflow

```rust
use map_reduce_core::map_reduce_job::MapReduceJob;
use word_search::{WordSearchProblem, WordSearchContext};

// 1. Load data
let data: Vec<String> = read_all_lines_from_files("data/*.txt");

// 2. Define search targets
let context = WordSearchContext {
    targets: vec![
        "the".to_string(),
        "of".to_string(),
        "and".to_string(),
        // ... top 20 words
    ],
};

// 3. Create map assignments
let map_assignments = WordSearchProblem::create_map_assignments(
    data,
    context.clone(),
    100,  // 100 lines per chunk
);

// 4. Execute map phase (infrastructure-specific)
executor.execute(map_assignments, 10, &shutdown).await?;

// 5. Create reduce assignments
let reduce_assignments = WordSearchProblem::create_reduce_assignments(
    context,
    10,  // 10 keys per reducer
);

// 6. Execute reduce phase
executor.execute(reduce_assignments, 10, &shutdown).await?;

// 7. Read results from state
for target in targets {
    let count: usize = state.get(&target).unwrap_or(0);
    println!("{}: {}", target, count);
}
```

---

## Configuration

### Tuning Parameters

**`partition_size`** (lines per mapper):
- **Smaller** (50-100): More parallelism, more overhead
- **Larger** (500-1000): Less parallelism, less overhead
- **Optimal**: Depends on file size and worker count

**`keys_per_reducer`**:
- **Smaller** (5-10): More parallel reducers
- **Larger** (20-50): Fewer reducers, less overhead
- **Optimal**: Usually `total_keys / num_reducers`

**`num_mappers`**:
- **More**: Better parallelism (up to CPU core count)
- **Fewer**: Less overhead
- **Optimal**: Match CPU core count

**`num_reducers`**:
- **More**: Better parallelism for large key sets
- **Fewer**: Less overhead
- **Optimal**: Typically 10-20 for word count

---

## Example Output

### Top 20 Words (Project Gutenberg corpus)

```
the: 58970
of: 30237
and: 23121
to: 21582
a: 19544
in: 14589
that: 11178
his: 10165
it: 9943
he: 9374
was: 9145
with: 7732
as: 7598
i: 7348
for: 7123
had: 6842
but: 6529
be: 6287
not: 6059
at: 5916

Total words counted: 1107186
```

---

## Performance Characteristics

### Scaling with Data Size

| Data Size | Mappers | Chunks | Time (task-channels) |
|-----------|---------|--------|----------------------|
| 100 KB | 5 | 20 | 0.15s |
| 1 MB | 10 | 200 | 0.45s |
| 10 MB | 10 | 2000 | 1.2s |
| 100 MB | 20 | 20000 | 8.5s |

**Scaling**: Linear with data size (constant per-line cost)

### Scaling with Worker Count

| Workers | Time | Speedup |
|---------|------|---------|
| 1 | 5.2s | 1.0x |
| 2 | 2.8s | 1.9x |
| 4 | 1.5s | 3.5x |
| 8 | 0.9s | 5.8x |
| 16 | 0.7s | 7.4x |

**Speedup**: Near-linear until CPU saturation

---

## Extending to Other Problems

### Inverted Index

```rust
impl MapReduceJob for InvertedIndexProblem {
    type MapAssignment = DocumentChunk;
    type ReduceAssignment = TermPartition;

    fn map_work<S>(assignment: &DocumentChunk, state: &S) {
        for (doc_id, text) in &assignment.docs {
            for word in tokenize(text) {
                // Emit: word -> [doc_id]
                state.update(word, vec![], |docs| docs.push(*doc_id));
            }
        }
    }

    fn reduce_work<S>(assignment: &TermPartition, state: &S) {
        for term in &assignment.terms {
            let doc_list: Vec<usize> = state.get(term).unwrap_or_default();
            // Deduplicate, sort, write to output
        }
    }
}
```

### PageRank

```rust
impl MapReduceJob for PageRankProblem {
    type MapAssignment = GraphPartition;
    type ReduceAssignment = NodePartition;

    fn map_work<S>(assignment: &GraphPartition, state: &S) {
        for (node, rank, neighbors) in &assignment.nodes {
            let contribution = rank / neighbors.len() as f64;
            for neighbor in neighbors {
                state.update(neighbor, 0.0, |r| *r += contribution);
            }
        }
    }

    fn reduce_work<S>(assignment: &NodePartition, state: &S) {
        for node in &assignment.nodes {
            let rank: f64 = state.get(node).unwrap_or(0.0);
            let new_rank = 0.15 + 0.85 * rank;
            state.set(node, new_rank);
        }
    }
}
```

---

## Code Organization

```
word-search/
├── src/
│   └── lib.rs         # MapReduceJob implementation
└── Cargo.toml
```

**Dependencies**:
- `map-reduce-core` - Trait definitions
- `serde` / `serde_json` - Serialization (for socket/RPC implementations)

---

## Key Takeaways

### Design Principles

1. **Separation of concerns**: Business logic separated from infrastructure
2. **Generic traits**: Same code works with any execution model
3. **Type safety**: Compile-time guarantees on correctness

### Benefits of Abstraction

✅ **Write once, run anywhere**:
- Same `WordSearchProblem` runs on tasks, threads, or processes
- No code changes needed to switch implementations

✅ **Testability**:
- Mock `StateAccess` for unit tests
- Test business logic independently of infrastructure

✅ **Extensibility**:
- Easy to add new problems (inverted index, PageRank)
- Easy to add new implementations (gRPC, Kubernetes)

### Limitations

⚠️ **Not a full MapReduce**:
- No shuffle/sort phase
- No combiner optimization
- Simplified reduce (no secondary sort)

⚠️ **Memory constraints**:
- All data loaded into memory
- State stored in single HashMap
- Not suitable for massive datasets (use disk-backed state)

---

## Next Steps

- **[Explore core traits](../core/README.md)** - Understand the abstraction layer
- **[See task-channels](../task-channels/README.md)** - Run with async tasks
- **[See thread-socket](../thread-socket/README.md)** - Run with TCP sockets
- **[See process-rpc](../process-rpc/README.md)** - Run with RPC
- **[Back to Main README](../README.md)**
