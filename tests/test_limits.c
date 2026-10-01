#include <assert.h>

#include "common.h"

int main(void)
{
    assert(MAX_REQUEST_LINE == 8192);
    assert(MAX_HEADER_BYTES == 16384);
    assert(MAX_HEADER_BYTES >= MAX_REQUEST_LINE);
    assert(MAX_BUFFERED_FILE == 64 * 1024 * 1024);
    return 0;
}
