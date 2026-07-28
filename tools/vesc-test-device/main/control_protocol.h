#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define CONTROL_MAX_LINE_BYTES 512
#define CONTROL_MAX_NOTIFY_BYTES 244

typedef enum {
    CONTROL_RESET,
    CONTROL_ADVERTISE,
    CONTROL_NOTIFY,
    CONTROL_DISCONNECT,
    CONTROL_LED,
} control_command_kind_t;

typedef struct {
    control_command_kind_t kind;
    uint8_t bytes[CONTROL_MAX_NOTIFY_BYTES];
    size_t length;
    uint8_t red;
    uint8_t green;
    uint8_t blue;
} control_command_t;

bool control_parse(const char *line, control_command_t *command);
bool control_decode_hex(
    const char *hex, uint8_t *destination, size_t capacity, size_t *length);
