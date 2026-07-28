#include "control_protocol.h"

#include <limits.h>
#include <string.h>

typedef enum {
    JSON_VALUE_NONE,
    JSON_VALUE_STRING,
    JSON_VALUE_U32,
    JSON_VALUE_BOOL,
} json_value_type_t;

typedef struct {
    json_value_type_t type;
    const char *start;
    size_t length;
    uint32_t number;
    bool boolean;
} json_value_t;

static void skip_spaces(const char **cursor) {
    while (**cursor == ' ' || **cursor == '\t') {
        (*cursor)++;
    }
}

static bool parse_flat_json_value(const char **cursor, json_value_t *value) {
    if (**cursor == '"') {
        (*cursor)++;
        value->type = JSON_VALUE_STRING;
        value->start = *cursor;
        while (**cursor != '\0' && **cursor != '"') {
            const unsigned char character = (unsigned char)**cursor;
            if (character < 0x20 || character > 0x7e || character == '\\') {
                return false;
            }
            (*cursor)++;
        }
        if (**cursor != '"') {
            return false;
        }
        value->length = (size_t)(*cursor - value->start);
        (*cursor)++;
        return true;
    }
    if (**cursor >= '0' && **cursor <= '9') {
        uint64_t number = 0;
        value->type = JSON_VALUE_U32;
        value->start = *cursor;
        while (**cursor >= '0' && **cursor <= '9') {
            number = number * 10 + (uint64_t)(**cursor - '0');
            if (number > UINT32_MAX) {
                return false;
            }
            (*cursor)++;
        }
        value->number = (uint32_t)number;
        value->length = (size_t)(*cursor - value->start);
        return true;
    }
    if (strncmp(*cursor, "true", 4) == 0) {
        value->type = JSON_VALUE_BOOL;
        value->boolean = true;
        value->length = 4;
        *cursor += 4;
        return true;
    }
    if (strncmp(*cursor, "false", 5) == 0) {
        value->type = JSON_VALUE_BOOL;
        value->boolean = false;
        value->length = 5;
        *cursor += 5;
        return true;
    }
    return false;
}

static bool json_find(
    const char *line, const char *wanted_key, json_value_t *wanted_value,
    bool *found) {
    const char *cursor = line;
    *found = false;
    skip_spaces(&cursor);
    if (*cursor++ != '{') {
        return false;
    }
    skip_spaces(&cursor);
    if (*cursor == '}') {
        cursor++;
        skip_spaces(&cursor);
        return *cursor == '\0';
    }

    while (*cursor != '\0') {
        if (*cursor++ != '"') {
            return false;
        }
        const char *key_start = cursor;
        while (*cursor != '\0' && *cursor != '"') {
            const unsigned char character = (unsigned char)*cursor;
            if (character < 0x20 || character > 0x7e ||
                character == '\\') {
                return false;
            }
            cursor++;
        }
        if (*cursor != '"') {
            return false;
        }
        const size_t key_length = (size_t)(cursor - key_start);
        cursor++;
        skip_spaces(&cursor);
        if (*cursor++ != ':') {
            return false;
        }
        skip_spaces(&cursor);
        json_value_t parsed = {0};
        if (!parse_flat_json_value(&cursor, &parsed)) {
            return false;
        }
        if (strlen(wanted_key) == key_length &&
            memcmp(key_start, wanted_key, key_length) == 0) {
            if (*found) {
                return false;
            }
            *wanted_value = parsed;
            *found = true;
        }
        skip_spaces(&cursor);
        if (*cursor == ',') {
            cursor++;
            skip_spaces(&cursor);
            continue;
        }
        if (*cursor == '}') {
            cursor++;
            skip_spaces(&cursor);
            return *cursor == '\0';
        }
        return false;
    }
    return false;
}

static int hex_nibble(char character) {
    if (character >= '0' && character <= '9') {
        return character - '0';
    }
    if (character >= 'a' && character <= 'f') {
        return character - 'a' + 10;
    }
    if (character >= 'A' && character <= 'F') {
        return character - 'A' + 10;
    }
    return -1;
}

bool control_decode_hex(
    const char *hex, uint8_t *destination, size_t capacity, size_t *length) {
    const size_t hex_length = strlen(hex);
    if (hex_length == 0 || (hex_length % 2) != 0 ||
        hex_length / 2 > capacity) {
        return false;
    }
    for (size_t index = 0; index < hex_length / 2; ++index) {
        const int high = hex_nibble(hex[index * 2]);
        const int low = hex_nibble(hex[index * 2 + 1]);
        if (high < 0 || low < 0) {
            return false;
        }
        destination[index] = (uint8_t)((high << 4) | low);
    }
    *length = hex_length / 2;
    return true;
}

bool control_json_u32(
    const char *line, const char *name, uint32_t *value) {
    json_value_t parsed = {0};
    bool found;
    if (!json_find(line, name, &parsed, &found) || !found ||
        parsed.type != JSON_VALUE_U32) {
        return false;
    }
    *value = parsed.number;
    return true;
}

bool control_json_string(
    const char *line, const char *name, char *value, size_t capacity) {
    json_value_t parsed = {0};
    bool found;
    if (!json_find(line, name, &parsed, &found) || !found ||
        parsed.type != JSON_VALUE_STRING || parsed.length + 1 > capacity) {
        return false;
    }
    memcpy(value, parsed.start, parsed.length);
    value[parsed.length] = '\0';
    return true;
}
