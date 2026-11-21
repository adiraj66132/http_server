#!/bin/sh
set -e

SERVER=${SERVER:-./http_server}
[ -x "$SERVER" ] || { echo "http_server not built" >&2; exit 1; }

if ./http_server --port abc >/dev/null 2>&1; then
    echo "expected nonzero exit for invalid port" >&2
    exit 1
fi

PORT=$((18000 + $$ % 1000))

"$SERVER" --port "$PORT" --root /nonexistent-httpd-root-$$ 2>/dev/null &
bad=$!
sleep 0.5
if kill -0 "$bad" 2>/dev/null; then
    kill "$bad" 2>/dev/null || true
    echo "expected startup failure for nonexistent document root" >&2
    exit 1
fi
wait "$bad" 2>/dev/null || true

"$SERVER" --port "$PORT" --root tests/fixtures/hello.txt 2>/dev/null &
bad=$!
sleep 0.5
if kill -0 "$bad" 2>/dev/null; then
    kill "$bad" 2>/dev/null || true
    echo "expected startup failure for file document root" >&2
    exit 1
fi
wait "$bad" 2>/dev/null || true

"$SERVER" --port "$PORT" --root . 2>/dev/null &
srv=$!
trap 'kill $srv 2>/dev/null || true; wait $srv 2>/dev/null || true' EXIT

python3 - "$PORT" <<'EOF'
import socket, sys, time

port = int(sys.argv[1])
for _ in range(50):
    try:
        s = socket.create_connection(("127.0.0.1", port), timeout=1)
        break
    except OSError:
        time.sleep(0.1)
else:
    sys.exit("server never listened")
s.settimeout(2)
s.sendall(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
data = b""
while True:
    chunk = s.recv(4096)
    if not chunk:
        break
    data += chunk
if not data.startswith(b"HTTP/1.1 "):
    sys.exit(f"expected HTTP response, got {data!r}")
s.close()
EOF

echo "smoke ok"
