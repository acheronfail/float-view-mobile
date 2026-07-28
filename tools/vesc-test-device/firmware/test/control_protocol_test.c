#include <assert.h>
#include <stdint.h>
#include <string.h>

#include "../main/control_protocol.h"

int main(void) {
    uint8_t bytes[4] = {0};
    size_t length = 0;
    assert(control_decode_hex("00aF10", bytes, sizeof(bytes), &length));
    assert(length == 3);
    assert(bytes[0] == 0x00 && bytes[1] == 0xaf && bytes[2] == 0x10);
    assert(!control_decode_hex("", bytes, sizeof(bytes), &length));
    assert(!control_decode_hex("0", bytes, sizeof(bytes), &length));
    assert(!control_decode_hex("0g", bytes, sizeof(bytes), &length));
    assert(!control_decode_hex("0011223344", bytes, sizeof(bytes), &length));

    const char *command =
        "{\"protocolVersion\":1,\"sequence\":42,\"kind\":\"notify\","
        "\"hex\":\"deadbeef\"}";
    uint32_t number = 0;
    char string[32];
    assert(control_json_u32(command, "protocolVersion", &number));
    assert(number == 1);
    assert(control_json_u32(command, "sequence", &number));
    assert(number == 42);
    assert(control_json_string(command, "kind", string, sizeof(string)));
    assert(strcmp(string, "notify") == 0);
    assert(control_json_string(command, "hex", string, sizeof(string)));
    assert(strcmp(string, "deadbeef") == 0);

    assert(!control_json_u32("{\"sequence\":4294967296}", "sequence", &number));
    assert(!control_json_u32("{\"sequence\":-1}", "sequence", &number));
    assert(!control_json_u32("{\"sequence\":1.5}", "sequence", &number));
    assert(!control_json_u32("{\"sequence\":1} trailing", "sequence", &number));
    assert(!control_json_u32(
        "{\"sequence\":1,\"sequence\":2}", "sequence", &number));
    assert(!control_json_string(
        "{\"kind\":\"escaped\\n\"}", "kind", string, sizeof(string)));
    assert(!control_json_string(
        "{\"kind\":{\"nested\":true}}", "kind", string, sizeof(string)));
    assert(!control_json_string(
        "{\"kind\":\"too-long\"}", "kind", string, 4));
    return 0;
}
