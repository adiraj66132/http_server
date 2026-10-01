#ifndef STATIC_FILES_H
#define STATIC_FILES_H

#include <stddef.h>

#include "parse.h"

int resolve_path(const char *docroot, const char *target, char *out,
                 size_t out_size);
const char *content_type_for(const char *path);
int handle_request(int fd, const struct http_request *req, const char *docroot);

#endif
