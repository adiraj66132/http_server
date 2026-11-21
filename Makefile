CC ?= cc
CFLAGS ?= -Wall -Wextra -std=c11 -g -Iinclude
LDLIBS ?= -lssl -lcrypto

SRCS := src/main.c src/startup.c src/parse.c src/response.c src/static_files.c \
	src/tls.c
HDRS := include/common.h include/startup.h include/parse.h include/response.h \
	include/static_files.h include/tls.h

TEST_BINS := tests/test_limits tests/test_startup tests/test_parse \
	tests/test_static

all: http_server

http_server: $(SRCS) $(HDRS)
	$(CC) $(CFLAGS) -o $@ $(SRCS) $(LDLIBS)

cert:
	openssl req -x509 -newkey rsa:2048 -sha256 -nodes -days 365 \
		-subj "/CN=localhost" -keyout key.pem -out cert.pem

test: $(TEST_BINS) http_server
	@set -e; for t in $(TEST_BINS); do ./$$t; done; \
		./tests/smoke_startup.sh; ./tests/smoke_verify.sh; \
		./tests/smoke_tls.sh; ./tests/smoke_shutdown.sh; \
		echo "all tests passed"

check: http_server
	./tests/check.sh

tests/test_limits: tests/test_limits.c include/common.h
	$(CC) $(CFLAGS) -o $@ $<

tests/test_startup: tests/test_startup.c src/startup.c include/startup.h
	$(CC) $(CFLAGS) -o $@ tests/test_startup.c src/startup.c

tests/test_parse: tests/test_parse.c src/parse.c src/tls.c include/parse.h \
		include/tls.h
	$(CC) $(CFLAGS) -o $@ tests/test_parse.c src/parse.c src/tls.c $(LDLIBS)

tests/test_static: tests/test_static.c src/static_files.c src/response.c \
		src/tls.c include/static_files.h include/response.h include/parse.h \
		include/tls.h
	$(CC) $(CFLAGS) -o $@ tests/test_static.c src/static_files.c \
		src/response.c src/tls.c $(LDLIBS)

clean:
	rm -f $(TEST_BINS) http_server

.PHONY: all cert test check clean
