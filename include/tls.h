#ifndef TLS_H
#define TLS_H

#include <stddef.h>
#include <openssl/ssl.h>

struct conn {
    int fd;
    SSL *ssl;
};

SSL_CTX *tls_init(const char *cert, const char *key);
SSL *tls_accept(SSL_CTX *ctx, int fd);

ssize_t conn_read(struct conn *c, void *buf, size_t n);
int conn_send_all(struct conn *c, const void *buf, size_t n);
void conn_close(struct conn *c);

#endif
