# Key-Value Server: In-Memory Implementation

Reference implementation of a distributed key-value store using **HashMap with Mutex** for storage.

## Overview

This is the simplest possible storage backend - all data lives in memory (lost on restart). It serves as:
1. **Reference implementation** for the `Storage` trait
2. **Performance baseline** for comparing other backends
3. **Testing vehicle** for protocol correctness

## Implementation

```rust
pub struct InMemoryStorage {
    data: Arc<Mutex<HashMap<String, (String, u64)>>>,
}
```

**Storage format**: `HashMap<Key, (Value, Version)>`

### Concurrency Model

- **Single global lock**: One `Mutex` protects the entire dataset
- **Trade-off**: Simple but limits concurrent throughput under high load
- **Good for**: Testing, development, low-to-medium concurrency workloads

## Building

```bash
cargo build --release --bin key-value-server-in-memory
```

## Running

```bash
cd key-value-server
cargo run --release --bin key-value-server-in-memory
```

The server:
1. Loads `config.json` from workspace root
2. Spawns configured clients (default: 5 clients with overlapping keys)
3. Runs stress test for configured duration (default: 30 seconds)
4. Auto-shuts down and prints final storage state

## Testing

Use the provided PowerShell script:

```powershell
cd server-in-memory/scripts
.\run_test.ps1
```

This:
- Builds the release binary
- Starts the server with embedded clients
- Waits for auto-shutdown
- Verifies graceful termination

## Expected Output

```
Loaded config: 5 clients, 30 second test duration, 3.0% packet loss
KV Server listening on 127.0.0.1:50051

[client_1][1] PUT 'key1' -> CREATE (value='value_123', new_version=1)
[client_2][1] PUT 'key1' -> KEY_EXISTS (fetching current version)
[client_2][1] PUT 'key1' -> UPDATE (value='value_456', new_version=2)
[SERVER] Simulating packet loss - dropping PUT response for key: key1
[client_2][2] PUT 'key1' -> NETWORK ERROR, retrying...
[client_2][2] PUT 'key1' -> RECOVERED after 1 network retry

30 seconds elapsed, initiating shutdown...

=== Final Storage State ===
  'key1' -> value='value_456', version=2
  ...
===========================
```

## Key Observations

### Version Conflicts
When multiple clients target the same key, you'll see:
- `KEY_EXISTS` → Client switches from create to update mode
- `VersionMismatch` → Client fetches latest version and retries
- Versions increment rapidly on hot keys

### Packet Loss Recovery
Watch for `RECOVERED` messages - these prove the client correctly detected that a "failed" write actually succeeded on the server.

### Cancellation
When shutdown triggers mid-operation:
```
[client_3][15] PUT 'key2' -> CANCELLED during network retry
```

## Performance Characteristics

| Metric | Value |
|--------|-------|
| Latency | ~100μs (local, no contention) |
| Throughput | ~10K ops/sec (5 concurrent clients) |
| Bottleneck | Single Mutex (all operations serialize) |

## Limitations

- **No persistence**: Data lost on restart
- **Single lock**: Limits scalability beyond ~10 concurrent clients
- **Memory only**: Dataset size limited by RAM

## Next Steps

Compare with:
- **Flat-file backend**: Durable but slower writes
- **Embedded-db backend**: Balanced performance + persistence
