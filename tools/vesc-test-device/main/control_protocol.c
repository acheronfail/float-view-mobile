#include "control_protocol.h"

#include <string.h>

static int nibble(char value) {
    if (value >= '0' && value <= '9') return value - '0';
    if (value >= 'A' && value <= 'F') return value - 'A' + 10;
    if (value >= 'a' && value <= 'f') return value - 'a' + 10;
    return -1;
}

bool control_decode_hex(
    const char *hex, uint8_t *destination, size_t capacity, size_t *length) {
    const size_t count = strlen(hex);
    if (count == 0 || count % 2 != 0 || count / 2 > capacity) return false;
    for (size_t index = 0; index < count / 2; index++) {
        const int high = nibble(hex[index * 2]);
        const int low = nibble(hex[index * 2 + 1]);
        if (high < 0 || low < 0) return false;
        destination[index] = (uint8_t)((high << 4) | low);
    }
    *length = count / 2;
    return true;
}

bool control_parse(const char *line, control_command_t *command) {
    if (line == NULL || command == NULL || strlen(line) > CONTROL_MAX_LINE_BYTES) {
        return false;
    }
    memset(command, 0, sizeof(*command));
    if (strcmp(line, "RESET") == 0) {
        command->kind = CONTROL_RESET;
        return true;
    }
    if (strcmp(line, "ADVERTISE") == 0) {
        command->kind = CONTROL_ADVERTISE;
        return true;
    }
    if (strcmp(line, "DISCONNECT") == 0) {
        command->kind = CONTROL_DISCONNECT;
        return true;
    }
    if (strncmp(line, "NOTIFY ", 7) == 0) {
        command->kind = CONTROL_NOTIFY;
        return control_decode_hex(
            line + 7, command->bytes, sizeof(command->bytes), &command->length);
    }
    if (strlen(line) == 10 && strncmp(line, "LED ", 4) == 0) {
        uint8_t rgb[3];
        size_t length;
        if (!control_decode_hex(line + 4, rgb, sizeof(rgb), &length) || length != 3) {
            return false;
        }
        command->kind = CONTROL_LED;
        command->red = rgb[0];
        command->green = rgb[1];
        command->blue = rgb[2];
        return true;
    }
    return false;
}
