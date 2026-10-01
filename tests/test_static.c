#define _DEFAULT_SOURCE

#include <assert.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>
#include <sys/socket.h>

#include "parse.h"
#include "static_files.h"

static char tmpdir[] = "/tmp/httpd_static_XXXXXX";
static char root[PATH_MAX];
static char outside[PATH_MAX];

static void write_file(const char *path, const char *content)
{
    FILE *f = fopen(path, "w");
    assert(f != NULL);
    fputs(content, f);
    fclose(f);
}

static void setup(void)
{
    assert(mkdtemp(tmpdir) != NULL);
    snprintf(root, sizeof(root), "%s/root", tmpdir);
    snprintf(outside, sizeof(outside), "%s/outside.txt", tmpdir);
    assert(mkdir(root, 0755) == 0);
    char path[PATH_MAX + 64];
    snprintf(path, sizeof(path), "%s/index.html", root);
    write_file(path, "hello\n");
    snprintf(path, sizeof(path), "%s/page.css", root);
    write_file(path, "body{}\n");
    snprintf(path, sizeof(path), "%s/pic.png", root);
    write_file(path, "PNGDATA");
    write_file(outside, "secret\n");
    snprintf(path, sizeof(path), "%s/link.txt", root);
    assert(symlink(outside, path) == 0);
}

static void cleanup(void)
{
    char path[PATH_MAX + 64];
    const char *files[] = {"index.html", "page.css", "pic.png", "link.txt"};
    for (size_t i = 0; i < sizeof(files) / sizeof(files[0]); i++) {
        snprintf(path, sizeof(path), "%s/%s", root, files[i]);
        unlink(path);
    }
    rmdir(root);
    unlink(outside);
    rmdir(tmpdir);
}

static size_t read_all(int fd, char *buf, size_t cap)
{
    size_t total = 0;
    ssize_t n;
    while ((n = read(fd, buf + total, cap - total - 1)) > 0)
        total += (size_t)n;
    buf[total] = '\0';
    return total;
}

static void make_req(struct http_request *req, const char *method,
                     const char *target)
{
    strcpy(req->method, method);
    strcpy(req->target, target);
    strcpy(req->version, "HTTP/1.1");
}

static size_t do_request(const struct http_request *req, char *resp,
                         size_t cap)
{
    int sv[2];
    assert(socketpair(AF_UNIX, SOCK_STREAM, 0, sv) == 0);
    assert(handle_request(sv[0], req, root) == 0);
    close(sv[0]);
    size_t n = read_all(sv[1], resp, cap);
    close(sv[1]);
    return n;
}

static long content_length(const char *resp)
{
    const char *p = strstr(resp, "Content-Length: ");
    return p ? atol(p + 16) : -1;
}

static void test_content_types(void)
{
    assert(strcmp(content_type_for("/x/index.html"), "text/html") == 0);
    assert(strcmp(content_type_for("/x/a.HTM"), "text/html") == 0);
    assert(strcmp(content_type_for("/x/a.css"), "text/css") == 0);
    assert(strcmp(content_type_for("/x/a.js"), "text/javascript") == 0);
    assert(strcmp(content_type_for("/x/a.json"), "application/json") == 0);
    assert(strcmp(content_type_for("/x/a.txt"), "text/plain") == 0);
    assert(strcmp(content_type_for("/x/a.png"), "image/png") == 0);
    assert(strcmp(content_type_for("/x/a.jpg"), "image/jpeg") == 0);
    assert(strcmp(content_type_for("/x/a.gif"), "image/gif") == 0);
    assert(strcmp(content_type_for("/x/a.svg"), "image/svg+xml") == 0);
    assert(strcmp(content_type_for("/x/a.ico"), "image/x-icon") == 0);
    assert(strcmp(content_type_for("/x/a.pdf"), "application/pdf") == 0);
    assert(strcmp(content_type_for("/x/a.unknown"), "application/octet-stream") == 0);
    assert(strcmp(content_type_for("/x/noext"), "application/octet-stream") == 0);
}

static void test_get_serves_file(void)
{
    struct http_request req;
    char resp[4096];
    make_req(&req, "GET", "/index.html");
    size_t n = do_request(&req, resp, sizeof(resp));
    assert(n > 0);
    assert(strncmp(resp, "HTTP/1.1 200 OK\r\n", 17) == 0);
    assert(strstr(resp, "Content-Type: text/html\r\n") != NULL);
    assert(content_length(resp) == 6);
    const char *body = strstr(resp, "\r\n\r\n");
    assert(body != NULL);
    body += 4;
    assert(strcmp(body, "hello\n") == 0);
}

static void test_head_no_body(void)
{
    struct http_request req;
    char resp[4096];
    make_req(&req, "HEAD", "/index.html");
    size_t n = do_request(&req, resp, sizeof(resp));
    assert(n > 0);
    assert(strncmp(resp, "HTTP/1.1 200 OK\r\n", 17) == 0);
    assert(content_length(resp) == 6);
    const char *body = strstr(resp, "\r\n\r\n");
    assert(body != NULL);
    assert(strcmp(body + 4, "") == 0);
}

static void test_404_missing(void)
{
    struct http_request req;
    char resp[4096];
    make_req(&req, "GET", "/nope.txt");
    do_request(&req, resp, sizeof(resp));
    assert(strncmp(resp, "HTTP/1.1 404 Not Found\r\n", 24) == 0);
    assert(content_length(resp) == 0);
}

static void test_404_directory(void)
{
    struct http_request req;
    char resp[4096];
    make_req(&req, "GET", "/");
    do_request(&req, resp, sizeof(resp));
    assert(strncmp(resp, "HTTP/1.1 404 Not Found\r\n", 24) == 0);
}

static void test_405_unsupported_method(void)
{
    struct http_request req;
    char resp[4096];
    make_req(&req, "POST", "/index.html");
    do_request(&req, resp, sizeof(resp));
    assert(strncmp(resp, "HTTP/1.1 405 Method Not Allowed\r\n", 33) == 0);
    assert(strstr(resp, "Allow: GET, HEAD\r\n") != NULL);
    assert(content_length(resp) == 0);
}

static void test_traversal_rejected(void)
{
    struct http_request req;
    char resp[4096];
    make_req(&req, "GET", "/../outside.txt");
    do_request(&req, resp, sizeof(resp));
    assert(strncmp(resp, "HTTP/1.1 404 Not Found\r\n", 24) == 0);

    make_req(&req, "GET", "/a/../../outside.txt");
    do_request(&req, resp, sizeof(resp));
    assert(strncmp(resp, "HTTP/1.1 404 Not Found\r\n", 24) == 0);
}

static void test_symlink_escape_rejected(void)
{
    struct http_request req;
    char resp[4096];
    make_req(&req, "GET", "/link.txt");
    do_request(&req, resp, sizeof(resp));
    assert(strncmp(resp, "HTTP/1.1 404 Not Found\r\n", 24) == 0);
}

static void test_resolve_normalizes(void)
{
    char out[PATH_MAX];
    char want[PATH_MAX + 64];
    char realroot[PATH_MAX];
    assert(realpath(root, realroot) != NULL);

    assert(resolve_path(root, "/index.html", out, sizeof(out)) == 0);
    snprintf(want, sizeof(want), "%s/index.html", realroot);
    assert(strcmp(out, want) == 0);

    assert(resolve_path(root, "/./a/../index.html", out, sizeof(out)) == 0);
    assert(strcmp(out, want) == 0);

    assert(resolve_path(root, "/index.html?x=1", out, sizeof(out)) == 0);
    assert(strcmp(out, want) == 0);

    assert(resolve_path(root, "//index.html", out, sizeof(out)) == 0);
    assert(strcmp(out, want) == 0);

    assert(resolve_path(root, "/../index.html", out, sizeof(out)) == -1);
    assert(resolve_path(root, "/a/../../index.html", out, sizeof(out)) == -1);
}

int main(void)
{
    setup();
    test_content_types();
    test_resolve_normalizes();
    test_get_serves_file();
    test_head_no_body();
    test_404_missing();
    test_404_directory();
    test_405_unsupported_method();
    test_traversal_rejected();
    test_symlink_escape_rejected();
    cleanup();
    printf("all tests passed\n");
    return 0;
}
