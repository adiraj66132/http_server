#include <errno.h>
#include <netinet/in.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

#include "startup.h"

static int parse_port(const char *s, int *out)
{
    char *end;
    long p = strtol(s, &end, 10);
    if (s[0] == '\0' || *end != '\0' || p < 1 || p > 65535)
        return -1;
    *out = (int)p;
    return 0;
}

int parse_args(int argc, char **argv, struct server_config *cfg)
{
    cfg->port = 8080;
    cfg->docroot = ".";

    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--port") == 0) {
            if (i + 1 >= argc) {
                fprintf(stderr, "missing value for --port\n");
                return -1;
            }
            i++;
            if (parse_port(argv[i], &cfg->port) != 0) {
                fprintf(stderr, "invalid port: %s\n", argv[i]);
                return -1;
            }
        } else if (strcmp(argv[i], "--root") == 0) {
            if (i + 1 >= argc) {
                fprintf(stderr, "missing value for --root\n");
                return -1;
            }
            i++;
            cfg->docroot = argv[i];
        } else {
            fprintf(stderr, "unknown option: %s\n", argv[i]);
            fprintf(stderr, "usage: %s [--port N] [--root DIR]\n", argv[0]);
            return -1;
        }
    }
    return 0;
}

int create_listener(int port)
{
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) {
        fprintf(stderr, "socket: %s\n", strerror(errno));
        return -1;
    }

    int one = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &one, sizeof(one));

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(INADDR_ANY);
    addr.sin_port = htons((unsigned short)port);

    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        fprintf(stderr, "bind: %s\n", strerror(errno));
        close(fd);
        return -1;
    }
    if (listen(fd, 64) < 0) {
        fprintf(stderr, "listen: %s\n", strerror(errno));
        close(fd);
        return -1;
    }
    return fd;
}
