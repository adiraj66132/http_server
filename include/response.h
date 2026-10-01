#ifndef RESPONSE_H
#define RESPONSE_H

#include <stddef.h>

const char *http_reason(int code);
int send_all(int fd, const void *buf, size_t n);
int send_error(int fd, int code);
int send_ok_head(int fd, const char *content_type, long long len);

#endif
