#include <assert.h>
#include <string.h>

#include "../main/control_protocol.h"

int main(void) {
    control_command_t command;
    assert(control_parse("RESET", &command) && command.kind == CONTROL_RESET);
    assert(control_parse("ADVERTISE", &command) && command.kind == CONTROL_ADVERTISE);
    assert(control_parse("DISCONNECT", &command) && command.kind == CONTROL_DISCONNECT);
    assert(control_parse("reset", &command) && command.kind == CONTROL_RESET);
    assert(control_parse("AdVeRtIsE", &command) && command.kind == CONTROL_ADVERTISE);
    assert(control_parse("disconnect", &command) && command.kind == CONTROL_DISCONNECT);

    assert(control_parse("NOTIFY 00aF10", &command));
    assert(command.kind == CONTROL_NOTIFY && command.length == 3);
    assert(command.bytes[0] == 0 && command.bytes[1] == 0xaf && command.bytes[2] == 0x10);
    assert(control_parse("nOtIfY 00Af10", &command));

    assert(control_parse("LED FF0080", &command));
    assert(command.kind == CONTROL_LED);
    assert(command.red == 0xff && command.green == 0 && command.blue == 0x80);
    assert(control_parse("LED 000000", &command));
    assert(control_parse("lEd ff0080", &command));

    assert(!control_parse("", &command));
    assert(!control_parse("NOTIFY ", &command));
    assert(!control_parse("NOTIFY 0", &command));
    assert(!control_parse("NOTIFY 0G", &command));
    assert(!control_parse("LED FFFFF", &command));
    assert(!control_parse("LED FFFFFF00", &command));
    assert(!control_parse("HELLO", &command));

    char edited[CONTROL_MAX_LINE_BYTES + 1] = {0};
    size_t edited_length = 0;
    const char delete_edit[] = {'R', 'E', 'S', 'E', 'X', 0x7f, 'T'};
    for (size_t index = 0; index < sizeof(delete_edit); index++) {
        assert(control_edit_line(edited, &edited_length, delete_edit[index]));
    }
    assert(strcmp(edited, "RESET") == 0);
    assert(control_parse(edited, &command) && command.kind == CONTROL_RESET);

    edited_length = 0;
    edited[0] = '\0';
    const char backspace_edit[] = {'A', 'D', 'X', '\b', 'V', 'E', 'R', 'T', 'I', 'S', 'E'};
    for (size_t index = 0; index < sizeof(backspace_edit); index++) {
        assert(control_edit_line(edited, &edited_length, backspace_edit[index]));
    }
    assert(strcmp(edited, "ADVERTISE") == 0);
    assert(control_parse(edited, &command) && command.kind == CONTROL_ADVERTISE);

    char oversized[CONTROL_MAX_LINE_BYTES + 2];
    memset(oversized, 'A', sizeof(oversized) - 1);
    oversized[sizeof(oversized) - 1] = '\0';
    assert(!control_parse(oversized, &command));
    return 0;
}
