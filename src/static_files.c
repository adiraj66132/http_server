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

int resolve_path(const char *docroot, const char *target, char *out,
                 size_t out_size)
{
    if (out_size < PATH_MAX)
        return -1;

    char rel[PATH_MAX];
    if (normalize(target, rel, sizeof(rel)) != 0)
        return -1;

    char candidate[PATH_MAX];
    int n = snprintf(candidate, sizeof(candidate), "%s/%s", docroot, rel);
    if (n < 0 || (size_t)n >= sizeof(candidate))
        return -1;

    char root_real[PATH_MAX];
    if (realpath(docroot, root_real) == NULL)
        return -1;
    if (realpath(candidate, out) == NULL)
        return -1;

    size_t rl = strlen(root_real);
    if (strncmp(out, root_real, rl) != 0)
        return -1;
    if (out[rl] != '\0' && out[rl] != '/')
        return -1;
    return 0;
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

static int serve_file(int fd, const char *path, int include_body)
{
    int f = open(path, O_RDONLY | O_NONBLOCK);
    if (f < 0)
        return send_error(fd, 404);

    struct stat st;
    if (fstat(f, &st) != 0 || !S_ISREG(st.st_mode)) {
        close(f);
        return send_error(fd, 404);
    }

    if (send_ok_head(fd, content_type_for(path), (long long)st.st_size) != 0) {
        close(f);
        return -1;
    }

    if (include_body) {
        char buf[65536];
        long long remaining = (long long)st.st_size;
        while (remaining > 0) {
            size_t chunk = remaining > (long long)sizeof(buf)
                               ? sizeof(buf)
                               : (size_t)remaining;
            ssize_t r = read(f, buf, chunk);
            if (r < 0) {
                if (errno == EINTR)
                    continue;
                close(f);
                return -1;
            }
            if (r == 0)
                break;
            if (send_all(fd, buf, (size_t)r) != 0) {
                close(f);
                return -1;
            }
            remaining -= r;
        }
    }

    close(f);
    return 0;
}

int handle_request(int fd, const struct http_request *req, const char *docroot)
{
    if (strcmp(req->method, "GET") != 0 && strcmp(req->method, "HEAD") != 0)
        return send_error(fd, 405);

    char path[PATH_MAX];
    if (resolve_path(docroot, req->target, path, sizeof(path)) != 0)
        return send_error(fd, 404);

    return serve_file(fd, path, strcmp(req->method, "HEAD") != 0);
}
