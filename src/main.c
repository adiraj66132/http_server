#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

#include "startup.h"

int main(int argc, char **argv)
{
    struct server_config cfg;
    if (parse_args(argc, argv, &cfg) != 0)
        return 1;

    int lfd = create_listener(cfg.port);
    if (lfd < 0)
        return 1;

    fprintf(stderr, "listening on port %d, root %s\n", cfg.port, cfg.docroot);

    for (;;) {
        int cfd = accept(lfd, NULL, NULL);
        if (cfd < 0) {
            if (errno == EINTR)
                continue;
            fprintf(stderr, "accept: %s\n", strerror(errno));
            break;
        }
        close(cfd);
    }

    close(lfd);
    return 0;
}
