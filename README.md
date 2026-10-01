# HTTP Server in C

A small, single-process HTTP/1.1 server for POSIX systems that serves static
files from a configured document root over TCP.

## Build

Requires a C compiler (`cc`/`gcc`), POSIX headers, and `make`:

```sh
make            # builds ./http_server
```

Without `make`:

```sh
cc -Wall -Wextra -std=c11 -g -Iinclude -o http_server \
    src/main.c src/startup.c src/parse.c src/response.c src/static_files.c
```

## Run

```sh
./http_server [--port N] [--root DIR]
```

| Option     | Default | Meaning                          |
|------------|---------|----------------------------------|
| `--port N` | `8080`  | TCP port to listen on (1-65535)  |
| `--root D` | `.`     | Document root to serve files from|

Example:

```sh
./http_server --port 8000 --root /var/www
curl http://127.0.0.1:8000/index.html
```

Invalid options, invalid ports, a bad document root (nonexistent or not a
directory), or socket failures print an error to stderr and exit non-zero.

## Supported behavior

- Methods: `GET` and `HEAD`. Any other valid method gets `405 Method Not
  Allowed` with an `Allow: GET, HEAD` header.
- One request per connection; the connection is closed after every response
  (`Connection: close`).
- `Content-Type` by extension for html/htm, css, js, json, txt, png, jpg/jpeg,
  gif, svg, ico, pdf, xml, wasm; anything else is `application/octet-stream`.
- Query strings (`/file.txt?x=1`) are ignored; the path is used.
- Status codes: `200`, `400` (malformed request), `404` (missing file,
  directory, or blocked path), `405`, `414` (request line too long), `431`
  (headers too large), `505` (unsupported HTTP version).
- Size limits: request line 8 KiB, total header block 16 KiB
  (`include/common.h`).
- HTTP/1.1 requests must carry exactly one `Host` header; missing or
  duplicated host is `400`, as are header values containing control bytes.
- Path traversal is blocked twice: `..` segments are normalized lexically and
  never allowed above the root, then the path is walked segment by segment
  from the document root with `openat(..., O_NOFOLLOW)` — a symlink anywhere
  in the path (even inside the root) yields `404`, and there is no window
  between path check and file open (no TOCTOU). This also makes `--root /`
  safe to serve absolute paths.
- Files up to 64 MiB are read fully before headers go out, so
  `Content-Length` always matches the bytes actually sent even if the file
  changes mid-response; larger files stream without `Content-Length`
  (close-delimited).
- Each accepted connection gets 5-second read/write timeouts: a client that
  stalls mid-request is answered `400` and disconnected, so one idle socket
  cannot freeze the server.
- `SIGINT`/`SIGTERM` shut the server down cleanly: the current connection
  finishes (a stalled one is cut off by the read timeout), the listening
  socket closes, exit status 0. Client disconnects and interrupted syscalls
  never crash the server.

## Tests

```sh
make test       # unit tests + smoke/verification scripts (needs curl, python3)
make check      # -Wpedantic build + ASan/UBSan run of everything
```

Tests exercise: arg parsing, socket binding, request parsing (partial reads,
limits, malformed input, Host rules, header value validation), static serving
(GET/HEAD, 404, 405, traversal, symlinks, root-`/` serving, large-file
framing), fd leaks, signal shutdown, and the full curl/raw-socket matrix.

## Known limitations

- No TLS, no persistent connections (keep-alive), no concurrency: requests are
  handled one at a time in a single process.
- No percent-decoding: a file reachable only as `%20`-encoded URL is not
  served; encoded traversal sequences are never decoded.
- No directory index: requesting a directory returns `404`.
- Symlinks are never followed, including ones inside the document root
  (serve real files there instead).
- No `Range`, conditional (`If-Modified-Since`), or caching headers.
- Response headers per HTTP/1.1 are sent regardless of whether the client
  asked for HTTP/1.0.
