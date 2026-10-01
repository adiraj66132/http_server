#include <errno.h>
#include <stddef.h>
#include <string.h>
#include <sys/socket.h>

#include "parse.h"

static const char *find_bytes(const char *hay, size_t n, const char *needle,
                              size_t needle_len)
{
    if (needle_len > n)
        return NULL;
    for (size_t i = 0; i + needle_len <= n; i++) {
        if (memcmp(hay + i, needle, needle_len) == 0)
            return hay + i;
    }
    return NULL;
}

static int is_tchar(unsigned char c)
{
    if ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
        (c >= '0' && c <= '9'))
        return 1;
    return strchr("!#$%&'*+-.^_`|~", c) != NULL && c != '\0';
}

static int valid_method(const char *s, size_t n)
{
    if (n == 0 || n >= 16)
        return 0;
    for (size_t i = 0; i < n; i++) {
        if (!is_tchar((unsigned char)s[i]))
            return 0;
    }
    return 1;
}

static int valid_header_name(const char *s, size_t n)
{
    if (n == 0)
        return 0;
    for (size_t i = 0; i < n; i++) {
        if (!is_tchar((unsigned char)s[i]))
            return 0;
    }
    return 1;
}

static int parse_headers(const char *start, const char *end)
{
    const char *pos = start;
    while (pos < end) {
        const char *nl = find_bytes(pos, (size_t)(end - pos), "\r\n", 2);
        const char *line_end = nl ? nl : end;
        const char *colon = memchr(pos, ':', (size_t)(line_end - pos));
        if (!colon || colon == pos)
            return 400;
        if (!valid_header_name(pos, (size_t)(colon - pos)))
            return 400;
        pos = nl ? nl + 2 : end;
    }
    return 0;
}

int parse_request(const char *buf, size_t len, struct http_request *req)
{
    const char *term = find_bytes(buf, len, "\r\n\r\n", 4);
    if (!term)
        return 400;

    const char *eol = find_bytes(buf, (size_t)(term - buf) + 2, "\r\n", 2);
    if (!eol)
        return 400;

    size_t line_len = (size_t)(eol - buf);
    if (line_len > MAX_REQUEST_LINE)
        return 414;

    const char *sp1 = memchr(buf, ' ', line_len);
    if (!sp1)
        return 400;
    const char *sp2 = memchr(sp1 + 1, ' ', (size_t)(eol - sp1 - 1));
    if (!sp2)
        return 400;
    if (memchr(sp2 + 1, ' ', (size_t)(eol - sp2 - 1)))
        return 400;

    size_t method_len = (size_t)(sp1 - buf);
    size_t target_len = (size_t)(sp2 - sp1 - 1);
    size_t version_len = (size_t)(eol - sp2 - 1);

    if (!valid_method(buf, method_len))
        return 400;
    if (target_len == 0 || sp1[1] != '/')
        return 400;
    for (size_t i = 0; i < target_len; i++) {
        unsigned char c = (unsigned char)sp1[1 + i];
        if (c < 0x21 || c == 0x7f)
            return 400;
    }

    char version[16];
    if (version_len >= sizeof(version))
        return 505;
    memcpy(version, sp2 + 1, version_len);
    version[version_len] = '\0';
    if (strcmp(version, "HTTP/1.0") != 0 && strcmp(version, "HTTP/1.1") != 0)
        return 505;

    int rc = parse_headers(eol + 2, term);
    if (rc != 0)
        return rc;

    memcpy(req->method, buf, method_len);
    req->method[method_len] = '\0';
    memcpy(req->target, sp1 + 1, target_len);
    req->target[target_len] = '\0';
    strcpy(req->version, version);
    return 0;
}

int read_request(int fd, struct http_request *req)
{
    char buf[MAX_HEADER_BYTES + 1];
    size_t total = 0;

    for (;;) {
        ssize_t n = recv(fd, buf + total, MAX_HEADER_BYTES - total, 0);
        if (n < 0) {
            if (errno == EINTR)
                continue;
            return 400;
        }
        if (n == 0)
            return 400;
        total += (size_t)n;

        if (find_bytes(buf, total, "\r\n\r\n", 4)) {
            buf[total] = '\0';
            return parse_request(buf, total, req);
        }
        if (total == MAX_HEADER_BYTES)
            return 431;
    }
}
