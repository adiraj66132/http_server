#define _POSIX_C_SOURCE 200809L

#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

#include "parse.h"
#include "response.h"
#include "static_files.h"
#include "startup.h"

static volatile sig_atomic_t running = 1;

static void on_signal(int sig)
{
    (void)sig;
    running = 0;
}

int main(int argc, char **argv)
{
    signal(SIGPIPE, SIG_IGN);

    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_handler = on_signal;
    sigemptyset(&sa.sa_mask);
    sigaction(SIGINT, &sa, NULL);
    sigaction(SIGTERM, &sa, NULL);

    struct server_config cfg;
    if (parse_args(argc, argv, &cfg) != 0)
        return 1;

    int lfd = create_listener(cfg.port);
    if (lfd < 0)
        return 1;

    fprintf(stderr, "listening on port %d, root %s\n", cfg.port, cfg.docroot);

    for (;;) {
        if (!running)
            break;
        int cfd = accept(lfd, NULL, NULL);
        if (cfd < 0) {
            if (errno == EINTR)
                continue;
            fprintf(stderr, "accept: %s\n", strerror(errno));
            break;
        }
        struct http_request req;
        int rc = read_request(cfd, &req);
        if (rc != 0)
            send_error(cfd, rc);
        else
            handle_request(cfd, &req, cfg.docroot);
        close(cfd);
    }

    close(lfd);
    return 0;
}
