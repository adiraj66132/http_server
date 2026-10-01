#!/bin/sh
set -e

CFLAGS="-Wall -Wextra -Wpedantic -std=c11 -g -Iinclude"
SAN="-fsanitize=address,undefined -fno-sanitize-recover=all"

echo "== pedantic syntax check =="
for f in src/*.c; do
    cc $CFLAGS -fsyntax-only "$f"
done

echo "== unit tests under ASan/UBSan =="
cc $CFLAGS $SAN -o tests/test_limits_asan tests/test_limits.c
cc $CFLAGS $SAN -o tests/test_startup_asan tests/test_startup.c src/startup.c
cc $CFLAGS $SAN -o tests/test_parse_asan tests/test_parse.c src/parse.c
cc $CFLAGS $SAN -o tests/test_static_asan tests/test_static.c \
    src/static_files.c src/response.c
./tests/test_limits_asan
./tests/test_startup_asan
./tests/test_parse_asan
./tests/test_static_asan

echo "== server under ASan/UBSan =="
cc $CFLAGS $SAN -o tests/http_server_asan src/*.c
SERVER=tests/http_server_asan ./tests/smoke_startup.sh
SERVER=tests/http_server_asan ./tests/smoke_verify.sh
SERVER=tests/http_server_asan ./tests/smoke_shutdown.sh

echo "check ok"
