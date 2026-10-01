#ifndef PARSE_H
#define PARSE_H

#include <stddef.h>

#include "common.h"

struct http_request {
    char method[16];
    char target[MAX_REQUEST_LINE + 1];
    char version[16];
};

int parse_request(const char *buf, size_t len, struct http_request *req);
int read_request(int fd, struct http_request *req);

#endif
