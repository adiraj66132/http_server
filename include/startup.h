#ifndef STARTUP_H
#define STARTUP_H

struct server_config {
    int port;
    const char *docroot;
};

int parse_args(int argc, char **argv, struct server_config *cfg);
int create_listener(int port);

#endif
