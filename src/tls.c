#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

#include <openssl/err.h>

#include "tls.h"

static void report_ssl(const char *what)
{
    fprintf(stderr, "tls: %s failed\n", what);
    ERR_print_errors_fp(stderr);
}

static int alpn_select_cb(SSL *ssl, const unsigned char **out,
                          unsigned char *outlen, const unsigned char *in,
                          unsigned int inlen, void *arg)
{
    (void)ssl;
    (void)arg;
    static const unsigned char http11[] = "http/1.1";
    const unsigned int http11_len = sizeof(http11) - 1;

    for (unsigned int i = 0; i < inlen; i += (unsigned int)in[i] + 1) {
        unsigned int len = in[i];
        if (i + 1 + len > inlen)
            break;
        if (len == http11_len && memcmp(in + i + 1, http11, len) == 0) {
            *out = in + i + 1;
            *outlen = (unsigned char)len;
            return SSL_TLSEXT_ERR_OK;
        }
    }
    return SSL_TLSEXT_ERR_NOACK;
}

SSL_CTX *tls_init(const char *cert, const char *key)
{
    SSL_CTX *ctx = SSL_CTX_new(TLS_server_method());
    if (ctx == NULL) {
        report_ssl("SSL_CTX_new");
        return NULL;
    }
    if (SSL_CTX_set_min_proto_version(ctx, TLS1_2_VERSION) != 1) {
        report_ssl("setting minimum TLS version");
        SSL_CTX_free(ctx);
        return NULL;
    }
    SSL_CTX_set_options(ctx, SSL_OP_NO_COMPRESSION);
    SSL_CTX_set_alpn_select_cb(ctx, alpn_select_cb, NULL);

    if (SSL_CTX_use_certificate_chain_file(ctx, cert) != 1) {
        fprintf(stderr, "tls: cannot load certificate %s\n", cert);
        ERR_print_errors_fp(stderr);
        SSL_CTX_free(ctx);
        return NULL;
    }
    if (SSL_CTX_use_PrivateKey_file(ctx, key, SSL_FILETYPE_PEM) != 1) {
        fprintf(stderr, "tls: cannot load private key %s\n", key);
        ERR_print_errors_fp(stderr);
        SSL_CTX_free(ctx);
        return NULL;
    }
    if (SSL_CTX_check_private_key(ctx) != 1) {
        fprintf(stderr, "tls: certificate and key do not match\n");
        SSL_CTX_free(ctx);
        return NULL;
    }
    return ctx;
}

SSL *tls_accept(SSL_CTX *ctx, int fd)
{
    SSL *ssl = SSL_new(ctx);
    if (ssl == NULL)
        return NULL;
    if (SSL_set_fd(ssl, fd) != 1 || SSL_accept(ssl) != 1) {
        SSL_free(ssl);
        return NULL;
    }
    return ssl;
}

ssize_t conn_read(struct conn *c, void *buf, size_t n)
{
    if (c->ssl != NULL)
        return (ssize_t)SSL_read(c->ssl, buf, (int)n);
    return recv(c->fd, buf, n, 0);
}

int conn_send_all(struct conn *c, const void *buf, size_t n)
{
    const char *p = buf;
    while (n > 0) {
        ssize_t w;
        if (c->ssl != NULL) {
            int sw = SSL_write(c->ssl, p, (int)n);
            if (sw <= 0)
                return -1;
            w = sw;
        } else {
            w = send(c->fd, p, n, 0);
            if (w < 0) {
                if (errno == EINTR)
                    continue;
                return -1;
            }
        }
        p += w;
        n -= (size_t)w;
    }
    return 0;
}

void conn_close(struct conn *c)
{
    if (c->ssl != NULL) {
        SSL_shutdown(c->ssl);
        SSL_free(c->ssl);
        c->ssl = NULL;
        close(c->fd);
        return;
    }
    shutdown(c->fd, SHUT_WR);
    char buf[4096];
    while (recv(c->fd, buf, sizeof(buf), MSG_DONTWAIT) > 0)
        ;
    close(c->fd);
}
