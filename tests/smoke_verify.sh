#!/bin/sh
set -e

SERVER=${SERVER:-./http_server}
[ -x "$SERVER" ] || { echo "http_server not built" >&2; exit 1; }

PORT=$((20000 + $$ % 1000))
"$SERVER" --port "$PORT" --root tests/fixtures 2>/dev/null &
srv=$!
trap 'if [ -n "$srv" ]; then kill "$srv" 2>/dev/null || true; fi' EXIT

base="http://127.0.0.1:$PORT"
fail() { echo "FAIL: $1" >&2; exit 1; }

i=0
while [ $i -lt 50 ]; do
    curl -s -o /dev/null --max-time 1 "$base/hello.txt" && break
    i=$((i + 1))
    sleep 0.1
done
curl -s -o /dev/null --max-time 1 "$base/hello.txt" || fail "server not ready"

f=$(mktemp)
trap 'if [ -n "$srv" ]; then kill "$srv" 2>/dev/null || true; fi; rm -f "$f"' EXIT

code=$(curl -s -o "$f" -w '%{http_code}' "$base/hello.txt")
[ "$code" = "200" ] || fail "GET status $code"
grep -qx "hello" "$f" || fail "GET body"

ctype=$(curl -s -o /dev/null -w '%{content_type}' "$base/hello.txt")
[ "$ctype" = "text/plain" ] || fail "Content-Type $ctype"

clen=$(curl -sI "$base/hello.txt" | tr -d '\r' | awk -F': ' \
    'tolower($1) == "content-length" { print $2 }')
[ "$clen" = "6" ] || fail "Content-Length $clen"

code=$(curl -s -o "$f" -w '%{http_code}' "$base/missing.txt")
[ "$code" = "404" ] || fail "missing file status $code"

code=$(curl -sI -o "$f" -w '%{http_code}' "$base/hello.txt")
[ "$code" = "200" ] || fail "HEAD status $code"

code=$(curl -s -o "$f" -w '%{http_code}' -X POST "$base/hello.txt")
[ "$code" = "405" ] || fail "POST status $code"
allow=$(curl -s -X POST -D - -o /dev/null "$base/hello.txt" | tr -d '\r' | \
    grep -i '^Allow:')
case "$allow" in
    *GET*HEAD*) ;;
    *) fail "Allow header: $allow" ;;
esac

code=$(curl -s -o /dev/null -w '%{http_code}' -X DELETE "$base/hello.txt")
[ "$code" = "405" ] || fail "DELETE status $code"

code=$(curl -s -o /dev/null -w '%{http_code}' "$base/hello.txt?x=1")
[ "$code" = "200" ] || fail "query string status $code"

python3 - "$PORT" <<'EOF' || fail "raw socket checks"
import socket, sys

port = int(sys.argv[1])

def req(data):
    s = socket.create_connection(("127.0.0.1", port), timeout=3)
    s.sendall(data)
    out = b""
    while True:
        c = s.recv(4096)
        if not c:
            break
        out += c
    s.close()
    return out

def status(resp):
    if not resp:
        raise AssertionError("empty response")
    return resp.split(b"\r\n", 1)[0].decode()

r = req(b"GARBAGE\r\n\r\n")
assert "400 Bad Request" in status(r), f"malformed: {status(r)!r}"

r = req(b"GET / HTTP/1.1\r\nNoColonHere\r\n\r\n")
assert "400 Bad Request" in status(r), f"bad header: {status(r)!r}"

r = req(b"GET / HTTP/1.1\r\n\r\n")
assert "400 Bad Request" in status(r), f"no host: {status(r)!r}"

r = req(b"GET / HTTP/1.1\r\nHost: x\r\nHost: y\r\n\r\n")
assert "400 Bad Request" in status(r), f"dup host: {status(r)!r}"

r = req(b"GET / HTTP/1.1\r\nHost: x\r\nX-Bad: a\x01b\r\n\r\n")
assert "400 Bad Request" in status(r), f"ctrl byte: {status(r)!r}"

big = b"GET / HTTP/1.1\r\nX-Big: " + b"A" * 20000 + b"\r\n\r\n"
r = req(big)
assert "431 Request Header Fields Too Large" in status(r), f"oversized: {r[:60]!r}"

r = req(b"GET /../../hello.txt HTTP/1.1\r\nHost: x\r\n\r\n")
assert "404 Not Found" in status(r), f"traversal: {status(r)!r}"

r = req(b"GET /%2e%2e/hello.txt HTTP/1.1\r\nHost: x\r\n\r\n")
assert "404 Not Found" in status(r), f"encoded traversal: {status(r)!r}"

r = req(b"HEAD /hello.txt HTTP/1.1\r\nHost: x\r\n\r\n")
head, sep, body = r.partition(b"\r\n\r\n")
assert head.startswith(b"HTTP/1.1 200 OK"), f"HEAD: {head!r}"
assert b"Content-Length: 6" in head, f"HEAD CL: {head!r}"
assert sep and body == b"", f"HEAD body: {body!r}"

r = req(b"GET /hello.txt HTTP/1.0\r\n\r\n")
assert "200 OK" in status(r), f"HTTP/1.0: {status(r)!r}"

r = req(b"GET /nope.txt HTTP/1.1\r\nHost: x\r\n\r\n")
head, sep, body = r.partition(b"\r\n\r\n")
assert head.startswith(b"HTTP/1.1 404 Not Found"), f"404: {head!r}"
assert body == b"", f"404 body: {body!r}"

print("raw checks ok")
EOF

echo "verify ok"
