#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>

#include "response.h"

const char *http_reason(int code)
{
    switch (code) {
    case 200: return "OK";
    case 400: return "Bad Request";
    case 404: return "Not Found";
    case 405: return "Method Not Allowed";
    case 414: return "URI Too Long";
    case 431: return "Request Header Fields Too Large";
    case 505: return "HTTP Version Not Supported";
    default:  return "Error";
    }
}

int send_all(int fd, const void *buf, size_t n)
{
    const char *p = buf;
    while (n > 0) {
        ssize_t w = send(fd, p, n, 0);
        if (w < 0) {
            if (errno == EINTR)
                continue;
            return -1;
        }
        p += w;
        n -= (size_t)w;
    }
    return 0;
}

int send_error(int fd, int code)
{
    char buf[256];
    int n;
    if (code == 405)
        n = snprintf(buf, sizeof(buf),
                     "HTTP/1.1 %d %s\r\n"
                     "Allow: GET, HEAD\r\n"
                     "Connection: close\r\n"
                     "Content-Length: 0\r\n"
                     "\r\n",
                     code, http_reason(code));
    else
        n = snprintf(buf, sizeof(buf),
                     "HTTP/1.1 %d %s\r\n"
                     "Connection: close\r\n"
                     "Content-Length: 0\r\n"
                     "\r\n",
                     code, http_reason(code));
    if (n < 0 || (size_t)n >= sizeof(buf))
        return -1;
    return send_all(fd, buf, (size_t)n);
}

int send_ok_head(int fd, const char *content_type, long long len)
{
    char buf[512];
    int n = snprintf(buf, sizeof(buf),
                     "HTTP/1.1 200 OK\r\n"
                     "Content-Type: %s\r\n"
                     "Content-Length: %lld\r\n"
                     "Connection: close\r\n"
                     "\r\n",
                     content_type, len);
    if (n < 0 || (size_t)n >= sizeof(buf))
        return -1;
    return send_all(fd, buf, (size_t)n);
}
