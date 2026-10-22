#!/bin/sh
set -e

SERVER=${SERVER:-./http_server}
[ -x "$SERVER" ] || { echo "http_server not built" >&2; exit 1; }
command -v openssl >/dev/null || { echo "openssl CLI required" >&2; exit 1; }

tmp=$(mktemp -d)
srv=
cleanup()
{
    if [ -n "$srv" ]; then kill "$srv" 2>/dev/null || true; fi
    rm -rf "$tmp"
}
trap cleanup EXIT

openssl req -x509 -newkey rsa:2048 -sha256 -nodes -days 1 \
    -subj "/CN=first" -keyout "$tmp/key.pem" -out "$tmp/cert.pem" \
    >/dev/null 2>&1

if timeout 2 "$SERVER" --port 65000 --cert "$tmp/cert.pem" >/dev/null 2>&1; then
    echo "expected failure when --key is missing" >&2
    exit 1
fi
if timeout 2 "$SERVER" --port 65000 --cert "$tmp/nope.pem" \
        --key "$tmp/key.pem" >/dev/null 2>&1; then
    echo "expected failure for unreadable certificate" >&2
    exit 1
fi

PORT=$((21000 + $$ % 1000))
"$SERVER" --port "$PORT" --root tests/fixtures \
    --cert "$tmp/cert.pem" --key "$tmp/key.pem" 2>/dev/null &
srv=$!
base="https://127.0.0.1:$PORT"
fail() { echo "FAIL: $1" >&2; exit 1; }

code=000
i=0
while [ $i -lt 50 ]; do
    code=$(curl -sk -o /dev/null -w '%{http_code}' --max-time 1 \
        "$base/hello.txt" || true)
    [ "$code" = "200" ] && break
    i=$((i + 1))
    sleep 0.1
done
[ "$code" = "200" ] || fail "server not ready over TLS (code=$code)"

f=$(mktemp)
trap 'if [ -n "$srv" ]; then kill "$srv" 2>/dev/null || true; fi; \
    rm -f "$f"; rm -rf "$tmp"' EXIT

code=$(curl -sk -o "$f" -w '%{http_code}' "$base/hello.txt")
[ "$code" = "200" ] || fail "TLS GET status $code"
grep -qx "hello" "$f" || fail "TLS GET body"

ctype=$(curl -sk -o /dev/null -w '%{content_type}' "$base/hello.txt")
[ "$ctype" = "text/plain" ] || fail "TLS Content-Type $ctype"

code=$(curl -sk -o /dev/null -w '%{http_code}' "$base/missing.txt")
[ "$code" = "404" ] || fail "TLS 404 status $code"

code=$(curl -skI -o /dev/null -w '%{http_code}' "$base/hello.txt")
[ "$code" = "200" ] || fail "TLS HEAD status $code"

if curl -s -o /dev/null --max-time 2 "http://127.0.0.1:$PORT/hello.txt"; then
    fail "plain HTTP unexpectedly succeeded on TLS port"
fi

peer_subject()
{
    echo | timeout 5 openssl s_client -connect "127.0.0.1:$PORT" \
        -servername localhost 2>/dev/null \
        | openssl x509 -noout -subject 2>/dev/null | tr -d '\r'
}

alpn=$(echo | timeout 5 openssl s_client -connect "127.0.0.1:$PORT" \
    -servername localhost -alpn http/1.1 2>/dev/null \
    | sed -n 's/^ALPN protocol: //p' | tr -d '\r')
[ "$alpn" = "http/1.1" ] || fail "ALPN http/1.1 not selected (got '$alpn')"

alpn=$(echo | timeout 5 openssl s_client -connect "127.0.0.1:$PORT" \
    -servername localhost -alpn h2,http/1.1 2>/dev/null \
    | sed -n 's/^ALPN protocol: //p' | tr -d '\r')
[ "$alpn" = "http/1.1" ] || fail "ALPN h2 fallback not http/1.1 (got '$alpn')"

subj=$(peer_subject)
case "$subj" in
    *first*) ;;
    *) fail "unexpected initial certificate: $subj" ;;
esac

openssl req -x509 -newkey rsa:2048 -sha256 -nodes -days 1 \
    -subj "/CN=second" -keyout "$tmp/key2.pem" -out "$tmp/cert2.pem" \
    >/dev/null 2>&1
cp "$tmp/cert2.pem" "$tmp/cert.pem"
cp "$tmp/key2.pem" "$tmp/key.pem"
kill -HUP "$srv"

subj=
i=0
while [ $i -lt 50 ]; do
    subj=$(peer_subject)
    case "$subj" in *second*) break ;; esac
    i=$((i + 1))
    sleep 0.1
done
case "$subj" in
    *second*) ;;
    *) fail "certificate was not reloaded: $subj" ;;
esac

code=$(curl -sk -o /dev/null -w '%{http_code}' "$base/hello.txt")
[ "$code" = "200" ] || fail "server failed after certificate reload"

printf 'broken\n' > "$tmp/cert.pem"
kill -HUP "$srv"
sleep 0.3
code=$(curl -sk -o /dev/null -w '%{http_code}' "$base/hello.txt")
[ "$code" = "200" ] || fail "server failed after bad certificate reload"
subj=$(peer_subject)
case "$subj" in
    *second*) ;;
    *) fail "previous certificate lost after bad reload: $subj" ;;
esac

echo "tls ok"
