#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

bool control_decode_hex(
    const char *hex, uint8_t *destination, size_t capacity, size_t *length);

bool control_json_u32(
    const char *line, const char *name, uint32_t *value);

bool control_json_string(
    const char *line, const char *name, char *value, size_t capacity);
