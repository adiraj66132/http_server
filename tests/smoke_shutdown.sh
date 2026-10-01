#!/bin/sh
set -e

SERVER=${SERVER:-./http_server}
[ -x "$SERVER" ] || { echo "http_server not built" >&2; exit 1; }

PORT=$((19000 + $$ % 1000))
"$SERVER" --port "$PORT" --root tests/fixtures 2>/dev/null &
srv=$!
trap 'if [ -n "$srv" ]; then kill "$srv" 2>/dev/null || true; fi; if [ -n "$idle" ]; then kill "$idle" 2>/dev/null || true; fi' EXIT

code=000
i=0
while [ $i -lt 50 ]; do
    code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 1 \
        "http://127.0.0.1:$PORT/hello.txt" || true)
    [ "$code" = "200" ] && break
    i=$((i + 1))
    sleep 0.1
done
[ "$code" = "200" ] || { echo "server never became ready (code=$code)"; exit 1; }

if [ -d "/proc/$srv/fd" ]; then
    before=$(ls "/proc/$srv/fd" | wc -l)
    i=0
    while [ $i -lt 100 ]; do
        curl -s -o /dev/null "http://127.0.0.1:$PORT/hello.txt"
        i=$((i + 1))
    done
    sleep 1
    after=$(ls "/proc/$srv/fd" | wc -l)
    if [ "$before" -ne "$after" ]; then
        echo "fd leak: before=$before after=$after"
        exit 1
    fi
fi

python3 - "$PORT" <<'EOF'
import socket, sys
s = socket.create_connection(("127.0.0.1", int(sys.argv[1])), timeout=2)
s.sendall(b"GET /hel")
s.close()
EOF
sleep 0.3
code=$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$PORT/hello.txt")
[ "$code" = "200" ] || { echo "server died after client disconnect"; exit 1; }

wait_for_exit()
{
    n=0
    while [ $n -lt 30 ]; do
        kill -0 "$srv" 2>/dev/null || return 0
        n=$((n + 1))
        sleep 0.1
    done
    return 1
}

python3 - "$PORT" <<'EOF'
import socket, sys
s = socket.create_connection(("127.0.0.1", int(sys.argv[1])), timeout=3)
s.settimeout(8)
data = s.recv(100)
assert data.startswith(b"HTTP/1.1 400"), \
    f"expected 400 timeout response, got {data!r}"
s.close()
EOF
code=$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$PORT/hello.txt")
[ "$code" = "200" ] || { echo "server unusable after idle client"; exit 1; }

python3 - "$PORT" <<'EOF' &
import socket, sys, time
s = socket.create_connection(("127.0.0.1", int(sys.argv[1])), timeout=3)
time.sleep(60)
EOF
idle=$!
sleep 0.5

kill -INT "$srv"
if ! wait_for_exit; then
    echo "server did not exit after SIGINT"
    exit 1
fi
set +e
wait "$srv"
status=$?
set -e
srv=
[ "$status" -eq 0 ] || { echo "SIGINT exit status $status, expected 0"; exit 1; }
kill "$idle" 2>/dev/null || true
idle=

"$SERVER" --port "$PORT" --root tests/fixtures 2>/dev/null &
srv=$!
i=0
while [ $i -lt 50 ]; do
    code=$(curl -s -o /dev/null -w '%{http_code}' --max-time 1 \
        "http://127.0.0.1:$PORT/hello.txt" || true)
    [ "$code" = "200" ] && break
    i=$((i + 1))
    sleep 0.1
done
[ "$code" = "200" ] || { echo "second server never became ready"; exit 1; }

kill -TERM "$srv"
if ! wait_for_exit; then
    echo "server did not exit after SIGTERM"
    exit 1
fi
set +e
wait "$srv"
status=$?
set -e
srv=
[ "$status" -eq 0 ] || { echo "SIGTERM exit status $status, expected 0"; exit 1; }

echo "shutdown smoke ok"
