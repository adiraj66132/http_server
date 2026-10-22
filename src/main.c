#define _DEFAULT_SOURCE
#define _POSIX_C_SOURCE 200809L

#include <errno.h>
#include <limits.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <unistd.h>

#include "parse.h"
#include "response.h"
#include "static_files.h"
#include "startup.h"
#include "tls.h"

#define CLIENT_IO_TIMEOUT_SEC 5

static volatile sig_atomic_t running = 1;
static volatile sig_atomic_t reload_certs = 0;

static void on_signal(int sig)
{
    (void)sig;
    running = 0;
}

static void on_hup(int sig)
{
    (void)sig;
    reload_certs = 1;
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
    sa.sa_handler = on_hup;
    sigaction(SIGHUP, &sa, NULL);

    struct server_config cfg;
    if (parse_args(argc, argv, &cfg) != 0)
        return 1;

    char root_real[PATH_MAX];
    struct stat root_st;
    if (realpath(cfg.docroot, root_real) == NULL ||
        stat(root_real, &root_st) != 0 || !S_ISDIR(root_st.st_mode)) {
        fprintf(stderr, "invalid document root: %s\n", cfg.docroot);
        return 1;
    }

    SSL_CTX *tls = NULL;
    if (cfg.cert != NULL) {
        tls = tls_init(cfg.cert, cfg.key);
        if (tls == NULL)
            return 1;
    }

    int lfd = create_listener(cfg.port);
    if (lfd < 0) {
        SSL_CTX_free(tls);
        return 1;
    }

    fprintf(stderr, "listening on port %d, root %s (%s)\n", cfg.port,
            cfg.docroot, tls != NULL ? "https" : "http");

    for (;;) {
        if (reload_certs) {
            reload_certs = 0;
            if (tls != NULL) {
                SSL_CTX *fresh = tls_init(cfg.cert, cfg.key);
                if (fresh != NULL) {
                    SSL_CTX_free(tls);
                    tls = fresh;
                    fprintf(stderr, "tls: reloaded certificate\n");
                } else {
                    fprintf(stderr,
                            "tls: reload failed, keeping previous certificate\n");
                }
            }
        }
        if (!running)
            break;
        int cfd = accept(lfd, NULL, NULL);
        if (cfd < 0) {
            if (errno == EINTR)
                continue;
            fprintf(stderr, "accept: %s\n", strerror(errno));
            break;
        }
        struct timeval tv = {CLIENT_IO_TIMEOUT_SEC, 0};
        setsockopt(cfd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));
        setsockopt(cfd, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof(tv));

        struct conn c = {cfd, NULL};
        if (tls != NULL) {
            c.ssl = tls_accept(tls, cfd);
            if (c.ssl == NULL) {
                close(cfd);
                continue;
            }
        }

        struct http_request req;
        int rc = read_request(&c, &req);
        if (rc != 0)
            send_error(&c, rc);
        else
            handle_request(&c, &req, cfg.docroot);
        conn_close(&c);
    }

    close(lfd);
    SSL_CTX_free(tls);
    return 0;
}
