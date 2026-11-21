#ifndef RESPONSE_H
#define RESPONSE_H

#include <stddef.h>

#include "tls.h"

const char *http_reason(int code);
int send_all(struct conn *c, const void *buf, size_t n);
int send_error(struct conn *c, int code);
int send_ok_head(struct conn *c, const char *content_type, long long len);

#endif
