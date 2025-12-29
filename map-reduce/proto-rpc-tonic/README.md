# Map-Reduce with Proto-RPC-Tonic (gRPC)

This implementation uses **Tonic** (gRPC for Rust) for communication between the coordinator and worker processes.

## Features

- **gRPC/Protocol Buffers** for efficient binary communication
- **Type-safe RPC** with code generation from `.proto` files
- **HTTP/2** with multiplexing and flow control
- **Production-ready** transport layer

## Architecture

- **State Server**: gRPC server managing shared state
- **Work Distribution**: gRPC streaming for work assignments
- **Completion Signaling**: gRPC unary calls for worker completion
- **Process Workers**: Separate processes spawned for mappers/reducers

## Running

```bash
cargo run --release
```

## Configuration

Edit `config.json` to adjust:
- Number of mappers/reducers
- Timeout values
- Failure/straggler probabilities
