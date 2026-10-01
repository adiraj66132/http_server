#define _DEFAULT_SOURCE

#include <assert.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/wait.h>
#include <unistd.h>

#include "common.h"
#include "parse.h"

static int parse(const char *s, struct http_request *req)
{
    return parse_request(s, strlen(s), req);
}

static void test_valid_get(void)
{
    struct http_request req;
    const char *msg = "GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n";
    assert(parse(msg, &req) == 0);
    assert(strcmp(req.method, "GET") == 0);
    assert(strcmp(req.target, "/index.html") == 0);
    assert(strcmp(req.version, "HTTP/1.1") == 0);
}

static void test_valid_head_http10(void)
{
    struct http_request req;
    const char *msg = "HEAD /a HTTP/1.0\r\n\r\n";
    assert(parse(msg, &req) == 0);
    assert(strcmp(req.method, "HEAD") == 0);
    assert(strcmp(req.target, "/a") == 0);
    assert(strcmp(req.version, "HTTP/1.0") == 0);
}

static void test_no_headers(void)
{
    struct http_request req;
    assert(parse("GET / HTTP/1.0\r\n\r\n", &req) == 0);
}

static void test_malformed_request_line(void)
{
    struct http_request req;
    assert(parse("GET\r\n\r\n", &req) == 400);
    assert(parse("GET /path\r\n\r\n", &req) == 400);
    assert(parse(" / HTTP/1.1\r\n\r\n", &req) == 400);
    assert(parse("GET  / HTTP/1.1\r\n\r\n", &req) == 400);
    assert(parse("GET / HTTP/1.1 extra\r\n\r\n", &req) == 400);
}

static void test_target_must_be_origin_form(void)
{
    struct http_request req;
    assert(parse("GET example.com HTTP/1.1\r\n\r\n", &req) == 400);
    assert(parse("GET /bad\ttab HTTP/1.1\r\n\r\n", &req) == 400);
}

static void test_bad_version(void)
{
    struct http_request req;
    assert(parse("GET / HTTP/2.0\r\n\r\n", &req) == 505);
    assert(parse("GET / HTTP/0.9\r\n\r\n", &req) == 505);
    assert(parse("GET / FTP/1.1\r\n\r\n", &req) == 505);
}

static void test_malformed_headers(void)
{
    struct http_request req;
    assert(parse("GET / HTTP/1.1\r\nNoColonHere\r\n\r\n", &req) == 400);
    assert(parse("GET / HTTP/1.1\r\n: empty name\r\n\r\n", &req) == 400);
    assert(parse("GET / HTTP/1.1\r\nBad Name: x\r\n\r\n", &req) == 400);
    assert(parse("GET / HTTP/1.1\r\nHost: ok\r\nBroken\r\n\r\n", &req) == 400);
}

static void test_long_request_line(void)
{
    struct http_request req;
    char msg[MAX_REQUEST_LINE + 64];
    size_t n = snprintf(msg, sizeof(msg), "GET /");
    memset(msg + n, 'a', MAX_REQUEST_LINE);
    snprintf(msg + n + MAX_REQUEST_LINE, 32, " HTTP/1.1\r\n\r\n");
    assert(parse(msg, &req) == 414);
}

static void test_valid_long_headers(void)
{
    struct http_request req;
    char msg[4096];
    size_t off = (size_t)snprintf(msg, sizeof(msg), "GET / HTTP/1.1\r\nHost: x\r\n");
    for (int i = 0; i < 50; i++)
        off += (size_t)snprintf(msg + off, sizeof(msg) - off,
                                "X-Header-%d: value\r\n", i);
    snprintf(msg + off, sizeof(msg) - off, "\r\n");
    assert(parse(msg, &req) == 0);
}

static void test_read_request_success(void)
{
    int sv[2];
    assert(socketpair(AF_UNIX, SOCK_STREAM, 0, sv) == 0);
    pid_t pid = fork();
    assert(pid >= 0);
    if (pid == 0) {
        close(sv[0]);
        const char *a = "GET /index.html HTTP/1.1\r\nHost: exam";
        const char *b = "ple.com\r\n\r\n";
        assert(write(sv[1], a, strlen(a)) == (ssize_t)strlen(a));
        usleep(100000);
        assert(write(sv[1], b, strlen(b)) == (ssize_t)strlen(b));
        close(sv[1]);
        _exit(0);
    }
    close(sv[1]);
    struct http_request req;
    assert(read_request(sv[0], &req) == 0);
    assert(strcmp(req.method, "GET") == 0);
    assert(strcmp(req.target, "/index.html") == 0);
    close(sv[0]);
    waitpid(pid, NULL, 0);
}

static void test_read_request_eof(void)
{
    int sv[2];
    assert(socketpair(AF_UNIX, SOCK_STREAM, 0, sv) == 0);
    const char *partial = "GET / HTTP/1.1\r\n";
    assert(write(sv[1], partial, strlen(partial)) == (ssize_t)strlen(partial));
    close(sv[1]);
    struct http_request req;
    assert(read_request(sv[0], &req) == 400);
    close(sv[0]);
}

static void test_read_request_oversized(void)
{
    int sv[2];
    assert(socketpair(AF_UNIX, SOCK_STREAM, 0, sv) == 0);
    char junk[MAX_HEADER_BYTES + 1024];
    memset(junk, 'A', sizeof(junk));
    assert(write(sv[1], junk, sizeof(junk)) == (ssize_t)sizeof(junk));
    struct http_request req;
    assert(read_request(sv[0], &req) == 431);
    close(sv[0]);
    close(sv[1]);
}

static void test_host_required_http11(void)
{
    struct http_request req;
    assert(parse("GET / HTTP/1.1\r\n\r\n", &req) == 400);
    assert(parse("GET / HTTP/1.1\r\nHost: a\r\n\r\n", &req) == 0);
    assert(parse("GET / HTTP/1.1\r\nHOST: a\r\n\r\n", &req) == 0);
    assert(parse("GET / HTTP/1.1\r\nHost: a\r\nHost: b\r\n\r\n", &req) == 400);
    assert(parse("GET / HTTP/1.0\r\n\r\n", &req) == 0);
}

static void test_header_value_control_bytes(void)
{
    struct http_request req;
    assert(parse("GET / HTTP/1.1\r\nHost: x\r\nX-A: a\x01"
                 "b\r\n\r\n", &req) == 400);
    assert(parse("GET / HTTP/1.1\r\nHost: a\nb\r\n\r\n", &req) == 400);
    assert(parse("GET / HTTP/1.1\r\nHost: a\rb\r\n\r\n", &req) == 400);
    assert(parse("GET / HTTP/1.1\r\nHost: x\r\nX-A: a\tb\r\n\r\n", &req) == 0);
    assert(parse("GET / HTTP/1.1\r\nHost: x\r\nX-A: \xc3\xa9\r\n\r\n",
                 &req) == 0);
    assert(parse("GET / HTTP/1.1\r\nHost: x\r\nX-A:\r\n\r\n", &req) == 0);
}

int main(void)
{
    test_valid_get();
    test_valid_head_http10();
    test_no_headers();
    test_malformed_request_line();
    test_target_must_be_origin_form();
    test_bad_version();
    test_malformed_headers();
    test_host_required_http11();
    test_header_value_control_bytes();
    test_long_request_line();
    test_valid_long_headers();
    test_read_request_success();
    test_read_request_eof();
    test_read_request_oversized();
    printf("all tests passed\n");
    return 0;
}
