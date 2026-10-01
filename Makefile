CC ?= cc
CFLAGS ?= -Wall -Wextra -std=c11 -g -Iinclude

SRCS := src/main.c src/startup.c src/parse.c
HDRS := include/common.h include/startup.h include/parse.h

TEST_BINS := tests/test_limits tests/test_startup tests/test_parse

all: http_server

http_server: $(SRCS) $(HDRS)
	$(CC) $(CFLAGS) -o $@ $(SRCS)

test: $(TEST_BINS) http_server
	@set -e; for t in $(TEST_BINS); do ./$$t; done; ./tests/smoke_startup.sh; echo "all tests passed"

tests/test_limits: tests/test_limits.c include/common.h
	$(CC) $(CFLAGS) -o $@ $<

tests/test_startup: tests/test_startup.c src/startup.c include/startup.h
	$(CC) $(CFLAGS) -o $@ tests/test_startup.c src/startup.c

tests/test_parse: tests/test_parse.c src/parse.c include/parse.h
	$(CC) $(CFLAGS) -o $@ tests/test_parse.c src/parse.c

clean:
	rm -f $(TEST_BINS) http_server

.PHONY: all test clean
