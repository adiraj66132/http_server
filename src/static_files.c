#define _DEFAULT_SOURCE

#include <ctype.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#include "response.h"
#include "static_files.h"

static int normalize(const char *target, char *rel, size_t rel_size)
{
    if (target[0] != '/')
        return -1;
    const char *q = strchr(target, '?');
    size_t tlen = q ? (size_t)(q - target) : strlen(target);
    size_t out = 0;
    size_t i = 1;

    while (i < tlen) {
        size_t start = i;
        while (i < tlen && target[i] != '/')
            i++;
        size_t seglen = i - start;
        if (i < tlen)
            i++;

        if (seglen == 0 || (seglen == 1 && target[start] == '.'))
            continue;
        if (seglen == 2 && target[start] == '.' && target[start + 1] == '.') {
            if (out == 0)
                return -1;
            while (out > 0 && rel[out - 1] != '/')
                out--;
            if (out > 0)
                out--;
            continue;
        }
        if (out > 0) {
            if (out + 1 >= rel_size)
                return -1;
            rel[out++] = '/';
        }
        if (out + seglen >= rel_size)
            return -1;
        memcpy(rel + out, target + start, seglen);
        out += seglen;
    }
    rel[out] = '\0';
    return 0;
}

static int open_under_root(const char *docroot, char *rel)
{
    int dirfd = open(docroot, O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    if (dirfd < 0)
        return -1;
    if (rel[0] == '\0') {
        close(dirfd);
        return -1;
    }

    int f = -1;
    char *p = rel;
    for (;;) {
        char *slash = strchr(p, '/');
        int last = (slash == NULL);
        if (slash != NULL)
            *slash = '\0';

        int flags = O_RDONLY | O_NOFOLLOW | O_CLOEXEC |
                    (last ? 0 : O_DIRECTORY);
        int next = openat(dirfd, p, flags);

        if (slash != NULL)
            *slash = '/';
        if (next < 0)
            break;
        if (last) {
            f = next;
            break;
        }
        close(dirfd);
        dirfd = next;
        p = slash + 1;
    }
    close(dirfd);
    return f;
}

static const struct {
    const char *ext;
    const char *type;
} types[] = {
    {".html", "text/html"},
    {".htm", "text/html"},
    {".css", "text/css"},
    {".js", "text/javascript"},
    {".json", "application/json"},
    {".txt", "text/plain"},
    {".png", "image/png"},
    {".jpg", "image/jpeg"},
    {".jpeg", "image/jpeg"},
    {".gif", "image/gif"},
    {".svg", "image/svg+xml"},
    {".ico", "image/x-icon"},
    {".pdf", "application/pdf"},
    {".xml", "application/xml"},
    {".wasm", "application/wasm"},
};

const char *content_type_for(const char *path)
{
    const char *dot = strrchr(path, '.');
    if (dot == NULL)
        return "application/octet-stream";

    char ext[16];
    size_t elen = strlen(dot);
    if (elen >= sizeof(ext))
        return "application/octet-stream";
    for (size_t i = 0; i < elen; i++)
        ext[i] = (char)tolower((unsigned char)dot[i]);
    ext[elen] = '\0';
    if (strchr(ext, '/') != NULL)
        return "application/octet-stream";

    for (size_t i = 0; i < sizeof(types) / sizeof(types[0]); i++) {
        if (strcmp(ext, types[i].ext) == 0)
            return types[i].type;
    }
    return "application/octet-stream";
}

static int send_file_body(int fd, int f, const struct stat *st,
                          const char *rel, int head_only)
{
    long long size = (long long)st->st_size;

    if (size > MAX_BUFFERED_FILE) {
        if (send_ok_head(fd, content_type_for(rel), -1) != 0)
            return -1;
        if (head_only)
            return 0;
        char buf[65536];
        for (;;) {
            ssize_t r = read(f, buf, sizeof(buf));
            if (r < 0) {
                if (errno == EINTR)
                    continue;
                return -1;
            }
            if (r == 0)
                return 0;
            if (send_all(fd, buf, (size_t)r) != 0)
                return -1;
        }
    }

    if (head_only)
        return send_ok_head(fd, content_type_for(rel), size);

    char *buf = NULL;
    size_t got = 0;
    if (size > 0) {
        buf = malloc((size_t)size);
        if (buf == NULL)
            return send_error(fd, 500);
        while (got < (size_t)size) {
            ssize_t r = read(f, buf + got, (size_t)size - got);
            if (r < 0) {
                if (errno == EINTR)
                    continue;
                break;
            }
            if (r == 0)
                break;
            got += (size_t)r;
        }
    }

    int rc = send_ok_head(fd, content_type_for(rel), (long long)got);
    if (rc == 0 && got > 0)
        rc = send_all(fd, buf, got);
    free(buf);
    return rc;
}

int handle_request(int fd, const struct http_request *req, const char *docroot)
{
    if (strcmp(req->method, "GET") != 0 && strcmp(req->method, "HEAD") != 0)
        return send_error(fd, 405);

    char rel[PATH_MAX];
    if (normalize(req->target, rel, sizeof(rel)) != 0)
        return send_error(fd, 404);

    int f = open_under_root(docroot, rel);
    if (f < 0)
        return send_error(fd, 404);

    struct stat st;
    if (fstat(f, &st) != 0 || !S_ISREG(st.st_mode)) {
        close(f);
        return send_error(fd, 404);
    }

    int rc = send_file_body(fd, f, &st, rel, strcmp(req->method, "HEAD") == 0);
    close(f);
    return rc;
}
