# HTTP Server in C

**Goal:** Small static file server for POSIX.
Single process. Takes a port and docroot.

**Scope:**
- `GET` and `HEAD` only, else 405
- One request per connection
- Size limits on headers
- No TLS, keep-alive, CGI, or async

**Steps**

1. **Setup:** `src/`, `include/`, `tests/`, Makefile, README.
   Separate modules for startup, parsing, responses, files.
   Shared limits in one place.
2. **Sockets:** Parse args, bind, listen.
   Accept loop. Clear errors.
3. **Parsing:** Read until the blank line.
   Handle partial reads. Cap header size.
   Validate the request line.
4. **Files:** Map URL to docroot. Block `../`.
   404 if missing. Set Content-Type.
   HEAD = same headers, no body.
5. **Shutdown:** Handle EINTR and disconnects.
   Close all fds.
   Exit cleanly on SIGINT/SIGTERM.
6. **Testing:** curl and browser.
   Try 404, 405, bad requests,
   big headers, traversal.
   Use `-Wall -Wextra` and ASan.
7. **Docs:** README with build, run,
   options, and limits.

**Milestone:** Safe `GET`/`HEAD` serving
with proper errors.