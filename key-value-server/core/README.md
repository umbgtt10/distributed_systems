# Key-Value Server Core

Core library for building distributed key-value stores with **optimistic concurrency control** and fault tolerance, inspired by MIT 6.824 Lab 3.

## Overview

This crate provides the foundational abstractions, gRPC protocol, client implementation, and server orchestration needed to build production-grade distributed KV stores with different storage backends.

## Inspiration

Based on MIT 6.824's approach to distributed systems:
- **Optimistic Concurrency Control (OCC)**: Version-based conflict detection instead of locks
- **Fault Injection**: Client and server-side packet loss simulation for testing
- **Recovery Detection**: Smart retry logic that recognizes when "failed" writes actually succeeded
- **Graceful Degradation**: Separate retry strategies for network errors vs application errors

## Stack

- **gRPC/Protocol Buffers**: Type-safe RPC with Tonic 0.14
- **Tokio**: Async runtime for concurrent client/server operations
- **Rust 2021**: Memory safety and fearless concurrency

## Key Abstractions

### 1. Storage Trait
```rust
#[async_trait]
pub trait Storage: Send + Sync {
    async fn get(&self, key: &str) -> Result<(String, u64), StorageError>;
    async fn put(&self, key: &str, value: String, expected_version: u64) -> Result<u64, StorageError>;
    async fn print_all(&self);
}
```
**Purpose**: Pluggable storage backends (in-memory, flat-file, embedded-db).

### 2. KeyValueServer
Generic gRPC service implementation that wraps any `Storage` and handles:
- Version conflict detection
- Error mapping (Storage → Protobuf)
- Request validation

### 3. PacketLossWrapper
Middleware that simulates network failures by dropping PUT responses after successful writes.
**Why**: Tests recovery logic - can clients detect that their "failed" write actually succeeded?

### 4. GrpcClient
Sophisticated client with:
- **Network retry logic**: Limited retries with exponential backoff for transient failures
- **Application retry logic**: Unlimited retries for version conflicts (fetch new version, retry)
- **Recovery detection**: Recognizes when a VersionMismatch indicates a previous write succeeded
- **Graceful shutdown**: Respects cancellation tokens mid-operation

### 5. ServerRunner
Zero-boilerplate orchestrator that:
- Spawns multiple concurrent clients from config
- Manages auto-shutdown timer and Ctrl+C handling
- Coordinates graceful termination
- Prints final storage state

## Protocol

Defined in `proto/key-value-server.proto`:

```protobuf
service KvService {
  rpc Get(GetRequest) returns (GetResponse);
  rpc Put(PutRequest) returns (PutResponse);
}
```

**PUT semantics**:
- `version=0` → Create (fails if key exists)
- `version=N` → Update (fails if current version ≠ N)

**Structured errors**:
- `VersionMismatch` includes `actual_version` field (no string parsing!)
- `KeyAlreadyExists`, `KeyNotFound` for clear failure modes

## Retry Logic

### Network Errors (Packet Loss)
- **Max retries**: Configurable (default 10)
- **Action**: Increment counter, sleep, retry same operation
- **Recovery**: If subsequent VersionMismatch has `actual_version != expected`, the previous write succeeded!

### Application Errors
- **Version conflicts**: Fetch latest version, retry indefinitely
- **KeyAlreadyExists**: Switch from create to update mode
- **KeyNotFound**: Switch from update to create mode

## What The Protocol Can Recover From

### ✅ Network Packet Loss
**Client-side**: Request never sent → Retry with counter, limited attempts (default: 10)

**Server-side**: Response dropped after successful write → Client retries, gets `VersionMismatch` with the new version from the succeeded write → **Smart recovery**: Client detects "my previous write worked!" and completes successfully

### ✅ Version Conflicts
Multiple clients writing to same key → Each gets `VersionMismatch` → Fetch latest version → Retry with correct version → **Unlimited retries** (protocol coordination, not failure)

### ✅ Create/Update Mode Mismatches
- `KeyAlreadyExists` during create → GET current version → Switch to update mode → Retry
- `KeyNotFound` during update → Switch to create mode → Retry

### ✅ Concurrent Operations
Optimistic concurrency naturally serializes conflicting writes:
1. Clients A and B both read `version=5`
2. A writes first → `version=6`
3. B fails with `VersionMismatch(actual=6)`
4. B fetches `version=6`, retries → `version=7`

No deadlocks, no explicit coordination.

### ✅ Graceful Shutdown
Clients check `cancellation_token` in retry loops → Exit cleanly mid-operation without orphaned connections

### ❌ Cannot Recover From
- **Server crashes** (no persistence in in-memory implementation)
- **Network partitions** (clients exhaust retries)
- **Byzantine failures** (no authentication/corruption detection)

**Key Insight**: The protocol distinguishes between **network failures** (limited retries) and **application coordination** (unlimited retries). Version conflicts aren't failures - they're the natural consequence of concurrent access.

## Configuration

JSON-based test scenarios (`config.json`):
```json
{
  "clients": [
    {
      "name": "client_1",
      "keys": ["key1", "key2"],
      "client_packet_loss_rate": 3.0
    }
  ],
  "test_duration_seconds": 30,
  "server_packet_loss_rate": 5.0,
  "max_retries_server_packet_loss": 10
}
```

## Usage

Implement `Storage`, pass to `ServerRunner`:

```rust
let storage = MyStorage::new();
ServerRunner::new(storage, "config.json", "127.0.0.1:50051")?
    .run()
    .await
```

That's it! The runner handles everything else.
