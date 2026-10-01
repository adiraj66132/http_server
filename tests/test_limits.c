#include <assert.h>

#include "common.h"

int main(void)
{
    assert(MAX_REQUEST_LINE == 8192);
    assert(MAX_HEADER_BYTES == 16384);
    assert(MAX_HEADER_BYTES >= MAX_REQUEST_LINE);
    return 0;
}
