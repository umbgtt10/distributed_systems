# process-rpc - Multi-Process RPC Implementation

**[← Back to MapReduce README](../README.md)**

This implementation uses **separate OS processes** with **RPC over TCP**, providing **true fault isolation**. Workers run as independent processes that can crash without affecting the coordinator, making this the **most realistic** implementation.

---

## Overview

**Execution Model**: Separate OS processes (spawned via `std::process::Command`)
**Communication**: RPC over TCP with length-delimited JSON
**State Storage**: `RpcStateAccess` (TCP client to remote state server)
**Isolation**: Process-level (true isolation, separate memory spaces)
**Performance**: ⭐ Variable (0.80-2.77s, process startup overhead)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  Coordinator Process (Main)                  │
│                         Executor                             │
└──────┬────────────────────────────────┬─────────────────────┘
       │                                │
  ┌────▼─────┐                     ┌────▼─────┐
  │  State   │                     │  RPC     │
  │  Server  │                     │ Listener │
  │  :9000   │                     │  :8001   │
  └────┬─────┘                     └────┬─────┘
       │                                │
       │                                │
       │                                │
  ┌────▼──────────────────────┐   ┌─────▼─────────────────┐
  │  Mapper Process 1         │   │  Reducer Process 1    │
  │  (spawned subprocess)     │   │  (spawned subprocess) │
  │                           │   │                       │
  │  - RPC client             │   │  - RPC client         │
  │  - State client           │   │  - State client       │
  │  - Runs independently     │   │  - Runs independently │
  └───────────────────────────┘   └───────────────────────┘
       │                                │
  ┌────▼───────────────────┐      ┌────▼──────────────────┐
  │  Mapper Process N      │      │  Reducer Process N    │
  └────────────────────────┘      └───────────────────────┘

```

**Key Features**:
- **Process isolation**: Each worker is a separate process
- **Independent crashes**: Worker crash doesn't affect coordinator
- **Remote state**: Workers connect to state server via TCP
- **True distribution**: Could run on separate machines (with network config)

---

## Key Components

### 1. `RpcWorkChannel` - RPC-Based Work Distribution

Uses length-delimited frames over TCP. Workers listen on assigned ports for work assignments from the coordinator.

**Protocol**: Length-delimited frames with JSON payloads containing serialized assignment and completion token.

**Characteristics**:
- **Length-delimited**: Uses `LengthDelimitedCodec` (prevents partial reads)
- **Async RPC**: Uses Tokio for non-blocking I/O
- **Connection reuse**: Each worker maintains persistent connection
- **Retry logic**: Handles connection failures gracefully

---

### 2. `RpcCompletionSignaling` - RPC Completion Tokens

Workers signal completion via length-delimited messages over TCP.

**Protocol**: Length-delimited frames with JSON `CompletionMessage` enum.

**Characteristics**:
- **Async accept**: Non-blocking RPC server
- **Structured messages**: `CompletionMessage` enum
- **Error handling**: Distinguishes success/failure/timeout

---

### 3. `ProcessRuntime` - OS Process Spawning

Spawns worker processes by executing the same binary with `--worker` flag and passing configuration via command-line arguments. Uses `AutoKillChild` wrapper for automatic cleanup.

**Characteristics**:
- **Same binary**: Workers are same executable with `--worker` flag
- **Argument passing**: Worker config via command-line args
- **Auto-cleanup**: `AutoKillChild` kills process on drop (prevents orphans)
- **Independent execution**: Worker has own memory, own crash domain

---

### 4. `RpcStateAccess` - Remote State Client

Workers connect to centralized state server via TCP and send length-prefixed JSON requests (`Initialize`, `Update`, `Replace`, `Get`). Connection is cached and reused across multiple requests.
- **Centralized state**: One state server, all workers connect
- **Network overhead**: Every operation is an RPC call
- **Simple protocol**: Length-prefixed JSON over TCP
- **Lazy connection**: Connects on first use, then reuses same connection

---

**State Server RPC Protocol**:
```rust
pub enum StateRequest {
    Initialize(Vec<String>),
    Update(String, i32),
    Replace(String, i32),
    Get(String),
}

pub enum StateResponse {
    Ok,
    Value(Vec<i32>),
    Error(String),
}
```

---

### 5. `StateServer` - Centralized State Server

Standalone server managing shared state with concurrent connection handling.

**Characteristics**:
- **Concurrent**: Handles multiple connections simultaneously
- **Thread-safe**: `Arc<Mutex<HashMap>>` for shared state
- **Persistent**: Runs for duration of MapReduce job
- **Centralized**: Single point of failure (could replicate for HA)

---

## Advantages

### ✅ True Fault Isolation

**Worker crashes don't affect coordinator**:
```rust
// Worker process crashes (simulated)
if should_fail {
    panic!("Simulated worker failure");
}
// Process exits with error code
// Coordinator detects via completion timeout
// Spawns replacement worker
```

**Benefits**:
- No shared memory corruption
- No poisoned locks
- Coordinator remains stable
- State server remains stable

### ✅ Most Realistic

**Closest to real distributed systems**:
- Independent processes
- Network communication
- Remote state access
- Process lifecycle management

**Easily distributed**:
- Change `127.0.0.1` to actual IPs
- Workers can run on different machines
- State server can be separate machine
- No code changes needed (just config)

### ✅ Advanced Features

**Fault injection**:
```json
{
  "mapper_failure_probability": 20,
  "reducer_failure_probability": 20,
  "mapper_straggler_probability": 10,
  "reducer_straggler_probability": 10
}
```

**Automatic cleanup**:
```rust
pub struct AutoKillChild {
    child: Child,
}

impl Drop for AutoKillChild {
    fn drop(&mut self) {
        let _ = self.child.kill();  // Ensure no orphans
    }
}
```

**Graceful shutdown**:
- Workers check shutdown signal
- StateServer responds to coordinator shutdown
- All processes cleaned up on exit

---

## Disadvantages

### ❌ Process Startup Overhead

**Process creation is expensive**:
- Windows: ~10-50ms per process
- Linux: ~5-20ms per process
- Includes: exec, memory allocation, dynamic linking

**Impact**:
- First run: High overhead (spawning 10 workers = ~100-500ms)
- Subsequent runs: Amortized over workload
- Many short jobs: Significant overhead

### ❌ Serialization Overhead (Doubled)

**Data serialized twice**:
1. Assignment → JSON (RPC)
2. State update → JSON → RPC → JSON

**Example**:
```
Word count update:
  Worker: state.update("hello", 1)
  → Serialize: StateRequest::Update("hello", 1)
  → RPC payload: { "Update": ["hello", 1] }
  → State server deserializes
  → Updates internal state: counts["hello"] += 1
```

**Cost**: ~50-200μs per RPC call

### ❌ Network Overhead

**Every operation is network I/O**:
- Work assignment: RPC call
- State access: RPC call (every initialize/update/replace/get)
- Completion: RPC call

**Comparison**:
- task-channels: 0 network calls (in-memory)
- thread-socket: 2-3 network calls per worker
- process-rpc: 10-100+ network calls per worker

### ❌ Complexity

**Most complex implementation**:
- Process management
- RPC protocol
- State server
- Connection management
- Error handling across process boundaries
- Argument parsing

---

## Configuration

**File**: `config.json`

```json
{
  "num_strings": 500000,
  "max_string_length": 15,
  "num_target_words": 100,
  "target_word_length": 3,
  "partition_size": 5000,
  "keys_per_reducer": 5,
  "num_mappers": 15,
  "num_reducers": 10,
  "mapper_failure_probability": 2,
  "reducer_failure_probability": 2,
  "mapper_straggler_probability": 2,
  "reducer_straggler_probability": 2,
  "mapper_straggler_delay_ms": 2000,
  "reducer_straggler_delay_ms": 2000,
  "mapper_timeout_ms": 10000,
  "reducer_timeout_ms": 10000
}
```

**Key Settings**:
- `num_strings` - Number of random strings to generate for input
- `max_string_length` - Maximum length of each generated string
- `num_target_words` - Number of target words to count (most frequent)
- `target_word_length` - Length of target words to generate
- `partition_size` - Number of strings per mapper assignment
- `keys_per_reducer` - Number of keys (words) per reducer assignment
- `num_mappers` / `num_reducers` - Number of worker processes
- `mapper_failure_probability` / `reducer_failure_probability` - Percent chance of worker crash (0-100)
- `mapper_straggler_probability` / `reducer_straggler_probability` - Percent chance of slow worker (0-100)
- `mapper_straggler_delay_ms` / `reducer_straggler_delay_ms` - Delay for stragglers
- `mapper_timeout_ms` / `reducer_timeout_ms` - Timeout before spawning backup worker

**Additional Notes**:
- State server and RPC addresses are dynamically allocated by OS (not in config)

---

## Running the Implementation

### Basic Run

```bash
cd process-rpc
cargo run
```

**Output**:
```
Starting MapReduce with process-rpc implementation...
State server started on 127.0.0.1:9000
RPC server started on 127.0.0.1:8001

Spawning 10 mapper processes...
Spawning 10 reducer processes...

Phase: Map
  Workers: 10
  Assignments: 1108
  Completed: 1108

Phase: Reduce
  Workers: 10
  Assignments: 10
  Completed: 10

Top 20 words:
the: 58970
of: 30237
...

Total execution time: 1.52s
```

### Monitoring Processes

```bash
# Windows
tasklist | findstr map-reduce-process-rpc

# Linux
ps aux | grep map-reduce-process-rpc
```

**You'll see**:
```
map-reduce-process-rpc.exe  12345  # Coordinator
map-reduce-process-rpc.exe  12346  # Mapper 1
map-reduce-process-rpc.exe  12347  # Mapper 2
...
```

### Monitoring Network

```bash
netstat -an | findstr 9000  # State server connections
netstat -an | findstr 8001  # RPC server connections
```

---

## Testing Fault Tolerance

### Worker Crashes

```json
{
  "mapper_failure_probability": 30,
  "reducer_failure_probability": 30,
  "mapper_timeout_ms": 3000,
  "reducer_timeout_ms": 3000
}
```

**Expected behavior**:
```
[INFO] Mapper 3 crashed (simulated failure)
[INFO] Timeout waiting for worker 3
[INFO] Spawning replacement worker 3
[INFO] Reassigning work to new worker 3
[INFO] Worker 3 completed successfully
```

**System remains stable**:
- Coordinator unaffected
- State server unaffected
- Other workers unaffected
- Work completed correctly

### Straggler Detection

```json
{
  "mapper_straggler_probability": 20,
  "reducer_straggler_probability": 20,
  "mapper_straggler_delay_ms": 10000,
  "reducer_straggler_delay_ms": 10000,
  "mapper_timeout_ms": 3000,
  "reducer_timeout_ms": 3000
}
```

**Expected behavior**:
```
[INFO] Worker 5 is straggler (10s delay)
[INFO] Timeout after 3s, spawning backup worker 5
[INFO] Both workers processing same work
[INFO] Backup worker 5 completed first
[INFO] Original worker 5 completed (result ignored)
```

---

## Performance Characteristics

**Performance Factors**:
- ❌ Process spawning: 10-50ms each
- ❌ Double serialization: JSON → RPC → JSON
- ❌ State RPC overhead: Every update is network call
- ❌ TCP connection setup
- ✅ True parallelism across processes
- ✅ No contention (separate memory spaces)

---

## Code Organization

```
process-rpc/
├── src/
│   ├── main.rs                       # Entry point + worker mode
│   ├── mapper.rs                     # Mapper implementation
│   ├── reducer.rs                    # Reducer implementation
│   ├── rpc_work_channel.rs           # RPC-based work distribution
│   ├── rpc_completion_signaling.rs   # RPC-based completion
│   ├── process_runtime.rs            # Process spawning + AutoKillChild
│   ├── rpc_state_access.rs           # RPC client for state
│   ├── state_server.rs               # Centralized state server
│   └── rpc.rs                        # RPC protocol helpers
├── config.json                       # Configuration
├── data/                             # Text files (not in repo)
└── Cargo.toml
```

---

## Advanced Features

### AutoKillChild - RAII Process Cleanup

Ensures no orphan processes through RAII - automatically kills child process when handle is dropped.

---

### Worker Mode Detection

Same binary acts as coordinator or worker depending on command-line arguments.

---

### Lazy Connection with Reuse

Establishes connection on first use and reuses it for all subsequent requests.
- Thread-safe through `Arc<Mutex<>>`

---

## Debugging Tips

### Enable Logging

```rust
// Add to Cargo.toml
env_logger = "0.11"

// In main.rs
env_logger::init();
log::info!("Worker {} starting", id);
log::debug!("Received assignment: {:?}", assignment);
```

### Inspect State Server

```bash
# Connect manually to state server
nc 127.0.0.1 9000

# Send RPC
{"method":"get","params":{"key":"the"}}
```

### Monitor Process Tree

```bash
# Linux
pstree -p | grep map-reduce

# Windows
wmic process where "name='map-reduce-process-rpc.exe'" get ProcessId,ParentProcessId
```

### Capture Network Traffic

```bash
# Wireshark filter
tcp.port == 9000 || tcp.port == 8001

# tcpdump
tcpdump -i lo -A 'port 9000 or port 8001'
```

---

## Key Takeaways

### When to Use

✅ **Perfect for**:
- Learning distributed systems concepts
- Testing fault tolerance mechanisms
- Process-level isolation requirements
- Realistic failure scenarios

❌ **Not suitable for**:
- Maximum performance requirements
- Very short-lived jobs (startup overhead)
- Systems with strict latency requirements
- Resource-constrained environments

### Design Lessons

1. **Processes are isolated**: Crashes truly don't propagate
2. **IPC is expensive**: Every boundary crossing has cost
3. **Cleanup matters**: Orphan processes are a real problem (RAII helps)
4. **Fault tolerance works**: Seeing workers crash and recover is educational

### Compared to Other Implementations

| Aspect | task-channels | thread-socket | **process-rpc** |
|--------|--------------|---------------|-----------------|
| Speed | Fastest | Moderate | **Variable** |
| Isolation | None | Thread-level | **Process-level** |
| Realism | Low | Medium | **High** |
| Complexity | Low | Medium | **High** |
| Fault Tolerance | ❌ | ⚠️ | **✅** |

---

## Next Steps

- **[Compare with task-channels](../task-channels/README.md)** - See async task implementation
- **[Compare with thread-socket](../thread-socket/README.md)** - See TCP socket implementation
- **[Explore core traits](../core/README.md)** - Understand the abstraction layer
- **[Back to Main README](../README.md)**
