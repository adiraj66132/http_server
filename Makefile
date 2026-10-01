CC ?= cc
CFLAGS ?= -Wall -Wextra -std=c11 -g -Iinclude

TEST_BINS := tests/test_limits

test: $(TEST_BINS)
	@set -e; for t in $(TEST_BINS); do ./$$t; done; echo "all tests passed"

tests/test_limits: tests/test_limits.c include/common.h
	$(CC) $(CFLAGS) -o $@ $<

clean:
	rm -f $(TEST_BINS)

.PHONY: test clean
