#include "control_protocol.h"

#include <string.h>

static int nibble(char value) {
    if (value >= '0' && value <= '9') return value - '0';
    if (value >= 'A' && value <= 'F') return value - 'A' + 10;
    if (value >= 'a' && value <= 'f') return value - 'a' + 10;
    return -1;
}

static char uppercase_ascii(char value) {
    if (value >= 'a' && value <= 'z') return (char)(value - ('a' - 'A'));
    return value;
}

static bool text_equal(const char *left, const char *right) {
    while (*left != '\0' && *right != '\0') {
        if (uppercase_ascii(*left++) != uppercase_ascii(*right++)) return false;
    }
    return *left == *right;
}

static bool text_starts_with(const char *text, const char *prefix) {
    while (*prefix != '\0') {
        if (*text == '\0' || uppercase_ascii(*text++) != uppercase_ascii(*prefix++)) {
            return false;
        }
    }
    return true;
}

bool control_edit_line(char *line, size_t *length, char value) {
    if (line == NULL || length == NULL || *length > CONTROL_MAX_LINE_BYTES) return false;
    if (value == '\b' || value == 0x7f) {
        if (*length > 0) line[--(*length)] = '\0';
        return true;
    }
    if (*length == CONTROL_MAX_LINE_BYTES) return false;
    line[(*length)++] = value;
    line[*length] = '\0';
    return true;
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
    if (text_equal(line, "RESET")) {
        command->kind = CONTROL_RESET;
        return true;
    }
    if (text_equal(line, "ADVERTISE")) {
        command->kind = CONTROL_ADVERTISE;
        return true;
    }
    if (text_equal(line, "DISCONNECT")) {
        command->kind = CONTROL_DISCONNECT;
        return true;
    }
    if (text_starts_with(line, "NOTIFY ")) {
        command->kind = CONTROL_NOTIFY;
        return control_decode_hex(
            line + 7, command->bytes, sizeof(command->bytes), &command->length);
    }
    if (strlen(line) == 10 && text_starts_with(line, "LED ")) {
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
