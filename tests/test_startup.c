#include <assert.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <unistd.h>

#include "startup.h"

static void test_defaults(void)
{
    char *argv[] = {"srv"};
    struct server_config cfg;
    assert(parse_args(1, argv, &cfg) == 0);
    assert(cfg.port == 8080);
    assert(strcmp(cfg.docroot, ".") == 0);
}

static void test_custom_options(void)
{
    char *argv[] = {"srv", "--port", "9090", "--root", "/var/www"};
    struct server_config cfg;
    assert(parse_args(5, argv, &cfg) == 0);
    assert(cfg.port == 9090);
    assert(strcmp(cfg.docroot, "/var/www") == 0);
}

static void test_bad_port_rejected(void)
{
    char *argv[] = {"srv", "--port", "abc"};
    struct server_config cfg;
    assert(parse_args(3, argv, &cfg) == -1);
}

static void test_port_out_of_range_rejected(void)
{
    char *argv1[] = {"srv", "--port", "0"};
    char *argv2[] = {"srv", "--port", "70000"};
    struct server_config cfg;
    assert(parse_args(3, argv1, &cfg) == -1);
    assert(parse_args(3, argv2, &cfg) == -1);
}

static void test_unknown_option_rejected(void)
{
    char *argv[] = {"srv", "--bogus"};
    struct server_config cfg;
    assert(parse_args(2, argv, &cfg) == -1);
}

static void test_missing_value_rejected(void)
{
    char *argv[] = {"srv", "--port"};
    struct server_config cfg;
    assert(parse_args(2, argv, &cfg) == -1);
}

static int local_port(int fd)
{
    struct sockaddr_in addr;
    socklen_t len = sizeof(addr);
    assert(getsockname(fd, (struct sockaddr *)&addr, &len) == 0);
    return ntohs(addr.sin_port);
}

static void test_listener_binds(void)
{
    int fd = create_listener(0);
    assert(fd >= 0);
    assert(local_port(fd) != 0);
    close(fd);
}

static void test_double_bind_fails(void)
{
    int fd = create_listener(0);
    assert(fd >= 0);
    int port = local_port(fd);
    assert(port != 0);
    assert(create_listener(port) == -1);
    close(fd);
}

int main(void)
{
    test_defaults();
    test_custom_options();
    test_bad_port_rejected();
    test_port_out_of_range_rejected();
    test_unknown_option_rejected();
    test_missing_value_rejected();
    test_listener_binds();
    test_double_bind_fails();
    printf("all tests passed\n");
    return 0;
}
