#define _DEFAULT_SOURCE

#include <assert.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/wait.h>
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

static size_t do_request_root(const struct http_request *req, char *resp,
                              size_t cap, const char *docroot)
{
    int sv[2];
    assert(socketpair(AF_UNIX, SOCK_STREAM, 0, sv) == 0);
    assert(handle_request(sv[0], req, docroot) == 0);
    close(sv[0]);
    size_t n = read_all(sv[1], resp, cap);
    close(sv[1]);
    return n;
}

static size_t do_request(const struct http_request *req, char *resp,
                         size_t cap)
{
    return do_request_root(req, resp, cap, root);
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

static void test_root_filesystem_serves(void)
{
    struct http_request req;
    char resp[4096];
    char target[PATH_MAX + 64];
    snprintf(target, sizeof(target), "%s/index.html", root);
    make_req(&req, "GET", target);
    size_t n = do_request_root(&req, resp, sizeof(resp), "/");
    assert(n > 0);
    assert(strncmp(resp, "HTTP/1.1 200 OK\r\n", 17) == 0);
    const char *body = strstr(resp, "\r\n\r\n");
    assert(body != NULL);
    assert(strcmp(body + 4, "hello\n") == 0);
}

static void test_normalize_via_serving(void)
{
    struct http_request req;
    char resp[4096];
    make_req(&req, "GET", "/./a/../index.html");
    do_request(&req, resp, sizeof(resp));
    assert(strncmp(resp, "HTTP/1.1 200 OK\r\n", 17) == 0);

    make_req(&req, "GET", "//index.html");
    do_request(&req, resp, sizeof(resp));
    assert(strncmp(resp, "HTTP/1.1 200 OK\r\n", 17) == 0);

    make_req(&req, "GET", "/index.html?x=1");
    do_request(&req, resp, sizeof(resp));
    assert(strncmp(resp, "HTTP/1.1 200 OK\r\n", 17) == 0);

    make_req(&req, "GET", "/a/../../index.html");
    do_request(&req, resp, sizeof(resp));
    assert(strncmp(resp, "HTTP/1.1 404 Not Found\r\n", 24) == 0);
}

static void test_inroot_symlinks_rejected(void)
{
    char linkf[PATH_MAX + 64], linkd[PATH_MAX + 64], sub[PATH_MAX + 64];
    char subfile[PATH_MAX + 128], inside[PATH_MAX + 64];
    snprintf(linkf, sizeof(linkf), "%s/rel.html", root);
    snprintf(inside, sizeof(inside), "%s/index.html", root);
    assert(symlink(inside, linkf) == 0);

    snprintf(sub, sizeof(sub), "%s/subdir", root);
    assert(mkdir(sub, 0755) == 0);
    snprintf(subfile, sizeof(subfile), "%s/f.txt", sub);
    write_file(subfile, "deep\n");
    snprintf(linkd, sizeof(linkd), "%s/linkdir", root);
    assert(symlink(sub, linkd) == 0);

    struct http_request req;
    char resp[4096];
    make_req(&req, "GET", "/rel.html");
    do_request(&req, resp, sizeof(resp));
    assert(strncmp(resp, "HTTP/1.1 404 Not Found\r\n", 24) == 0);

    make_req(&req, "GET", "/linkdir/f.txt");
    do_request(&req, resp, sizeof(resp));
    assert(strncmp(resp, "HTTP/1.1 404 Not Found\r\n", 24) == 0);

    unlink(linkf);
    unlink(linkd);
    unlink(subfile);
    rmdir(sub);
}

static void test_large_file_no_content_length(void)
{
    char path[PATH_MAX + 64];
    snprintf(path, sizeof(path), "%s/big.bin", root);
    FILE *f = fopen(path, "wb");
    assert(f != NULL);
    char zeros[65536];
    memset(zeros, 0, sizeof(zeros));
    size_t chunks = MAX_BUFFERED_FILE / sizeof(zeros) + 1;
    for (size_t i = 0; i < chunks; i++)
        assert(fwrite(zeros, 1, sizeof(zeros), f) == sizeof(zeros));
    fclose(f);
    size_t file_size = chunks * sizeof(zeros);
    assert(file_size > MAX_BUFFERED_FILE);

    int sv[2];
    assert(socketpair(AF_UNIX, SOCK_STREAM, 0, sv) == 0);
    pid_t pid = fork();
    assert(pid >= 0);
    if (pid == 0) {
        close(sv[1]);
        struct http_request req;
        make_req(&req, "GET", "/big.bin");
        handle_request(sv[0], &req, root);
        close(sv[0]);
        _exit(0);
    }
    close(sv[0]);

    char head[1024];
    size_t total = 0, head_len = 0;
    char buf[65536];
    ssize_t n;
    while ((n = read(sv[1], buf, sizeof(buf))) > 0) {
        if (head_len < sizeof(head) - 1) {
            size_t take = (size_t)n;
            if (head_len + take > sizeof(head) - 1)
                take = sizeof(head) - 1 - head_len;
            memcpy(head + head_len, buf, take);
            head_len += take;
        }
        total += (size_t)n;
    }
    close(sv[1]);
    waitpid(pid, NULL, 0);

    head[head_len] = '\0';
    char *term = strstr(head, "\r\n\r\n");
    assert(term != NULL);
    assert(strncmp(head, "HTTP/1.1 200 OK\r\n", 17) == 0);
    assert(strstr(head, "Content-Length:") == NULL);
    assert(total - (size_t)(term + 4 - head) == file_size);

    unlink(path);
}

int main(void)
{
    setup();
    test_content_types();
    test_root_filesystem_serves();
    test_normalize_via_serving();
    test_get_serves_file();
    test_head_no_body();
    test_404_missing();
    test_404_directory();
    test_405_unsupported_method();
    test_traversal_rejected();
    test_symlink_escape_rejected();
    test_inroot_symlinks_rejected();
    test_large_file_no_content_length();
    cleanup();
    printf("all tests passed\n");
    return 0;
}
