#ifndef STATIC_FILES_H
#define STATIC_FILES_H

#include "parse.h"

const char *content_type_for(const char *path);
int handle_request(int fd, const struct http_request *req, const char *docroot);

#endif
