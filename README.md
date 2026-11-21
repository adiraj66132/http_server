# HTTP/1.1 Server in Rust

A lightweight, standards-compliant HTTP/1.1 server implementation in Rust using Tokio. Built with zero `unsafe` code and no external HTTP libraries.

## Features

- **HTTP/1.1 Compliant**: Strict parsing per RFC 7230/7231
- **Async/Await**: Fully async using Tokio runtime
- **Concurrent Connections**: Semaphore-limited connection pool
- **Keep-Alive & Pipelining**: Persistent connections with request pipelining
- **Graceful Shutdown**: Stops accepting new connections, drains in-flight requests
- **Structured Logging**: Per-request logging with timing
- **Comprehensive Test Suite**: 76 tests (unit + integration)
- **Zero Unsafe**: `#![forbid(unsafe_code)]` enforced
- **No External HTTP Libraries**: Pure Rust implementation

## Quick Start

```bash
# Build
cargo build --release

# Run server (listens on 127.0.0.1:8080 by default)
cargo run --release

# Run tests
cargo test
```

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Returns "Hello, World!" (text/plain) |
| GET | `/health` | Returns `{"status":"ok"}` (application/json) |
| POST | `/echo` | Echoes request body with Content-Type |

## Configuration

```rust
Config {
    address: "127.0.0.1:8080".parse().unwrap(),
    max_connections: 1024,
    max_header_bytes: 8192,
    max_body_bytes: 1_048_576,
    header_read_timeout: Duration::from_secs(5),
    body_read_timeout: Duration::from_secs(30),
    shutdown_drain_timeout: Duration::from_secs(10),
}
```

## Architecture

```
src/
├── main.rs          # Entry point, signal handling
├── lib.rs           # Library root, module declarations
├── config.rs        # Configuration struct
├── parser.rs        # HTTP request parser (pure, no I/O)
├── router.rs        # Request routing & response building
├── response.rs      # HTTP response serialization
└── server.rs        # TCP server, connection handling
```

### Design Principles

1. **Separation of Concerns**: Parser, Router, Server are independent modules
2. **Pure Functions**: Parser and Router have no I/O, fully testable in isolation
3. **Error as Values**: Uses `thiserror` for typed errors
4. **Backpressure**: Semaphore limits concurrent connections
5. **Timeouts at Every Layer**: Header read, body read, shutdown drain

## Tools & Technologies

| Tool | Purpose |
|------|---------|
| **Rust 1.70+** | Systems programming language |
| **Tokio 1.x** | Async runtime, TCP, timeouts |
| **thiserror** | Derive error types |
| **cargo** | Build system, test runner |
| **clippy** | Linting |
| **rustfmt** | Formatting |

## Development Approach

### 1. Specification-Driven Development
- Defined requirements as test cases first (PR, FR, TS, RS prefixes)
- Each requirement maps to specific test(s)
- Traceability table in [TRACEABILITY.md](TRACEABILITY.md)

### 2. Test-First Implementation
- **Unit Tests** (44): Parser (33) + Router (11) - pure logic, no I/O
- **Integration Tests** (12): Real TCP connections against running server
- All 76 tests pass

### 3. Incremental Module Development
```
Phase 1: Parser (pure RFC 7230 parsing)
  → Request line, headers, body, validation

Phase 2: Router (pure request→response mapping)
  → Path matching, method dispatch, error handling

Phase 3: Response Serialization
  → HTTP/1.1 wire format, headers, status lines

Phase 4: Server (I/O, concurrency, timeouts)
  → TcpListener, per-connection tasks, semaphore
  → Graceful shutdown with listener drop

Phase 5: Integration & Polish
  → End-to-end tests, clippy, fmt, documentation
```

### 4. Quality Gates
- `cargo fmt --check` - enforced formatting
- `cargo clippy --all-targets -- -D warnings` - zero warnings
- `cargo test` - all tests must pass

## HTTP Compliance Details

### Request Parsing (RFC 7230)
- ✅ CRLF line endings required (bare LF → 400)
- ✅ HTTP/1.1 only (others → 505)
- ✅ Host header required (missing/duplicate → 400)
- ✅ Origin-form only (`/path?query`) (absolute/authority → 400)
- ✅ Header field validation (token chars only)
- ✅ Content-Length validation (numeric, non-negative, single)
- ✅ Transfer-Encoding: chunked → 501 (not implemented)
- ✅ Expect: 100-continue → 417

### Request Semantics (RFC 7231)
- ✅ GET / HEAD / POST / PUT / DELETE / OPTIONS
- ✅ 405 Method Not Allowed with Allow header
- ✅ 404 Not Found for unknown paths
- ✅ 404 takes precedence over 405
- ✅ No percent-decoding, no path normalization

### Connection Management
- ✅ Keep-alive by default (HTTP/1.1)
- ✅ Connection: close honored
- ✅ Pipelining: sequential processing, ordered responses
- ✅ Max header/body size limits (431/413)
- ✅ Read timeouts (408 Request Timeout)

### Response Format
- ✅ Status line: `HTTP/1.1 <code> <reason>`
- ✅ Content-Length on all responses
- ✅ Content-Type on all responses
- ✅ Connection: close on error responses (4xx/5xx)

## Testing

```bash
# All tests
cargo test

# Specific suites
cargo test --test parser_tests
cargo test --test router_tests
cargo test --test integration_tests

# With output
cargo test -- --nocapture

# Single test
cargo test ts4f_graceful_shutdown_in_flight_completes -- --nocapture
```

### Test Categories

| Suite | Tests | Coverage |
|-------|-------|----------|
| `parser_tests` | 33 | RFC 7230 parsing, validation, limits |
| `router_tests` | 20 | Routing, status codes, headers |
| `integration_tests` | 12 | Real TCP, concurrency, timeouts, shutdown |
| `lib.rs` (doc) | 11 | Internal router unit tests |

## Graceful Shutdown

The server implements proper graceful shutdown:

1. **Signal Received** (Ctrl+C / SIGTERM)
2. **Drop Listener** - New connections refused immediately (ECONNREFUSED)
3. **Drain In-Flight** - Wait for active requests to complete
4. **Timeout** - Force close after `shutdown_drain_timeout` (default 10s)
5. **Exit**

Test: `ts4f_graceful_shutdown_in_flight_completes` verifies new connections are rejected while in-flight request completes.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Project Structure

```
http-rust-server/
├── Cargo.toml              # Dependencies, metadata
├── Cargo.lock              # Locked dependencies
├── .gitignore              # Git ignore rules
├── README.md               # This file
├── LICENSE                 # MIT License
├── TRACEABILITY.md         # Requirements ↔ Implementation ↔ Tests
├── COMMANDS_AND_ASSUMPTIONS.md  # Commands, assumptions, limitations
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── config.rs
│   ├── parser.rs
│   ├── router.rs
│   ├── response.rs
│   └── server.rs
└── tests/
    ├── parser_tests.rs
    ├── router_tests.rs
    └── integration_tests.rs
```

## Running in Production

```bash
# Release build
cargo build --release

# Run with custom config (env vars or modify Config::default())
./target/release/http-server
```

The server binds to `127.0.0.1:8080` by default. For production, configure:
- Bind to `0.0.0.0` for external access
- Set appropriate `max_connections` for your system
- Consider adding TLS termination (nginx, traefik) in front
- Monitor logs for structured request logging