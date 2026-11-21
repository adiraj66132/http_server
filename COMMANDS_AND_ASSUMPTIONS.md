# Commands and Assumptions

## Commands

### Build and Test
```bash
# Build the project
cargo build

# Run all tests
cargo test

# Run specific test suites
cargo test --test integration_tests
cargo test --test parser_tests
cargo test --test router_tests

# Run specific test
cargo test ts4f_graceful_shutdown_in_flight_completes -- --nocapture
```

### Code Quality
```bash
# Check formatting
cargo fmt --check

# Format code
cargo fmt

# Run clippy (linter)
cargo clippy --all-targets -- -D warnings

# Run clippy with auto-fix
cargo fix --lib -p http-server
```

### Development
```bash
# Run server
cargo run

# Run with debug logging
RUST_LOG=debug cargo run
```

## Assumptions

### HTTP Protocol Assumptions
1. **HTTP/1.1 Only**: Server only supports HTTP/1.1. Other versions (HTTP/1.0, HTTP/2.0) return 505.
2. **CRLF Line Endings**: All request lines and headers must use CRLF (`\r\n`). Bare LF returns 400.
3. **Host Header Required**: HTTP/1.1 requests must include a Host header. Missing or duplicate returns 400.
4. **No Absolute/Authority Form**: Request targets must be origin-form (starting with `/`). Absolute-form (`http://...`) and authority-form (`host:port`) return 400.
5. **No Chunked Encoding**: Transfer-Encoding is not implemented. Requests with Transfer-Encoding return 501 unless Content-Length is also present (then 400).
6. **Content-Length Required for POST**: POST requests must have Content-Length. Missing returns 411.
7. **No Percent Decoding**: Path and query strings are not percent-decoded.
8. **No Trailing Slash Normalization**: `/health/` is different from `/health` (returns 404).
9. **Query String Ignored in Routing**: `/health?foo=bar` routes to `/health` handler.
10. **Case-Insensitive Header Names**: Header names are compared case-insensitively.
11. **Header Validation**: Header names must be valid tokens (no whitespace, no special chars like `@`, `(`, `)`, etc.).

### Server Behavior Assumptions
1. **One Task Per Connection**: Each TCP connection is handled in a separate Tokio task.
2. **Connection Limit**: Semaphore limits concurrent connections (configurable via `max_connections`).
3. **Keep-Alive by Default**: Connections are persistent unless client sends `Connection: close` or error occurs.
4. **Pipelining Support**: Multiple requests on same connection are processed sequentially in order.
4. **Timeouts**:
   - Header read timeout: 5s (configurable)
   - Body read timeout: 30s (configurable)
   - Graceful shutdown drain timeout: 10s (configurable)
5. **Max Header Size**: 8KB (configurable)
6. **Max Body Size**: 1MB (configurable)
7. **Graceful Shutdown**: On shutdown signal:
   - Stop accepting new connections (listener dropped)
   - Allow in-flight requests to complete
   - Wait for drain timeout before force-closing
8. **Structured Logging**: Each request logged as `<peer> <method> <path> <status> <duration_ms>`
9. **Error Responses Include Connection: Close**: All 4xx/5xx responses include `Connection: close`
10. **Success Responses**: No `Connection` header by default (keep-alive implied)

### Implementation Constraints
1. **No Unsafe Code**: `#![forbid(unsafe_code)]` enforced
2. **No External HTTP Libraries**: Pure Rust implementation using Tokio only
3. **Standard Library Only**: No `http`, `hyper`, or similar crates
4. **Async/Await**: Full async using Tokio

### Configuration Defaults
```rust
Config {
    address: "127.0.0.1:0".parse().unwrap(),
    max_connections: 1024,
    max_header_bytes: 8192,
    max_body_bytes: 1_048_576,
    header_read_timeout: Duration::from_secs(5),
    body_read_timeout: Duration::from_secs(30),
    shutdown_drain_timeout: Duration::from_secs(10),
}
```

### Test Assumptions
1. **Integration Tests**: Start real server on random port, make actual TCP connections
2. **Unit Tests**: Test parser and router in isolation with constructed Request objects
3. **Query String Handling**: Router tests manually set `query` field; parser tests verify splitting
4. **Graceful Shutdown Test**: Verifies new connections rejected after shutdown signal while in-flight completes

### Known Limitations
1. **No HTTPS/TLS**: Plain HTTP only
2. **No WebSocket Support**: HTTP upgrade not implemented
3. **No Request Body Streaming**: Full body buffered in memory
4. **No Response Streaming**: Full response built in memory
5. **Single-Threaded Accept Loop**: One accept loop, but connections handled concurrently