# HTTP Server in C — Build Plan

## Goal

Build a small, single-process HTTP/1.1 server in C for POSIX systems. It will serve static files from a configured document root over TCP.

## Initial scope

- Support `GET` and `HEAD` requests.
- Return `405 Method Not Allowed` for unsupported methods.
- Handle one request per connection, then close the connection.
- Enforce request and header size limits.
- Defer TLS, persistent connections, CGI, and asynchronous I/O.

## Implementation steps

### Task 1: Set up the project

Create a simple project layout with `src/`, `include/`, and `tests/` directories, along with a `Makefile` and `README.md`. Keep server startup, request parsing, response generation, and static-file serving in separate modules. Define shared limits and error handling centrally.

### Task 2: Implement socket startup

- Parse command-line options for the listening port and document root.
- Create a TCP socket, bind it, and begin listening.
- Report configuration and socket errors clearly.
- Accept connections in a straightforward loop.

### Task 3: Parse HTTP requests safely

- Read headers until the HTTP header terminator, handling partial reads.
- Set a maximum header size and reject requests that exceed it.
- Validate the request line and HTTP version.
- Parse headers without assuming they arrive in a single read.
- Return suitable errors for malformed requests.

### Task 4: Serve static files

- Map URL paths to files under the configured document root.
- Prevent directory traversal and ensure resolved files remain under that root.
- Return `404 Not Found` when a requested file does not exist.
- Set basic `Content-Type` values for common file extensions.
- Send a valid status line and response headers, followed by the file body for `GET`.
- For `HEAD`, send the same headers as `GET` but omit the body.

### Task 5: Handle errors and shutdown

- Handle interrupted system calls and client disconnects without crashing.
- Close sockets and file descriptors on all response paths.
- Support orderly shutdown on `SIGINT` and `SIGTERM`.

### Task 6: Verify behavior

Exercise the server with a browser or `curl`. Check:

- Successful `GET` and `HEAD` responses.
- Missing files and unsupported methods.
- Malformed requests and oversized headers.
- Path traversal attempts.
- Compiler and memory checks where available.

### Task 7: Document usage

Document build and run commands, options, supported behavior, and known limitations in `README.md`.

## First milestone

A server that safely serves static files for `GET` and `HEAD`, with basic HTTP error responses. Consider persistent connections or concurrency only if the intended use requires them.
