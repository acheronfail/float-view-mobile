#include <assert.h>
#include <string.h>

#include "../main/control_protocol.h"

int main(void) {
    control_command_t command;
    assert(control_parse("RESET", &command) && command.kind == CONTROL_RESET);
    assert(control_parse("ADVERTISE", &command) && command.kind == CONTROL_ADVERTISE);
    assert(control_parse("DISCONNECT", &command) && command.kind == CONTROL_DISCONNECT);

    assert(control_parse("NOTIFY 00aF10", &command));
    assert(command.kind == CONTROL_NOTIFY && command.length == 3);
    assert(command.bytes[0] == 0 && command.bytes[1] == 0xaf && command.bytes[2] == 0x10);

    assert(control_parse("LED FF0080", &command));
    assert(command.kind == CONTROL_LED);
    assert(command.red == 0xff && command.green == 0 && command.blue == 0x80);
    assert(control_parse("LED 000000", &command));

    assert(!control_parse("", &command));
    assert(!control_parse("NOTIFY ", &command));
    assert(!control_parse("NOTIFY 0", &command));
    assert(!control_parse("NOTIFY 0G", &command));
    assert(!control_parse("LED FFFFF", &command));
    assert(!control_parse("LED FFFFFF00", &command));
    assert(!control_parse("HELLO", &command));

    char oversized[CONTROL_MAX_LINE_BYTES + 2];
    memset(oversized, 'A', sizeof(oversized) - 1);
    oversized[sizeof(oversized) - 1] = '\0';
    assert(!control_parse(oversized, &command));
    return 0;
}
