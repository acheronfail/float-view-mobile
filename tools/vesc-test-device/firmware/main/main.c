/* T1 transport firmware for the Float View programmable VESC test device.
 * Project license is pending; see docs/implementation-scope-lock.md.
 */

#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "control_protocol.h"
#include "driver/usb_serial_jtag.h"
#include "esp_err.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"
#include "freertos/task.h"
#include "host/ble_att.h"
#include "host/ble_gatt.h"
#include "host/ble_gap.h"
#include "host/ble_hs.h"
#include "host/ble_uuid.h"
#include "host/util/util.h"
#include "nimble/ble.h"
#include "nimble/nimble_port.h"
#include "nvs_flash.h"
#include "os/os_mbuf.h"
#include "services/gap/ble_svc_gap.h"
#include "services/gatt/ble_svc_gatt.h"

#define CONTROL_PROTOCOL_VERSION 1
#define DEVICE_NAME "FloatView Test VESC"
#define PROFILE_NAME "canonicalVescUart"
#define MAX_CONTROL_LINE_BYTES 768
#define MAX_GATT_WRITE_BYTES 256
#define MAX_NOTIFY_BYTES 244
#define MAX_OUTPUT_LINE_BYTES 896
#define USB_IO_TIMEOUT_MS 20
#define HOST_WATCHDOG_MS 5000

void ble_store_config_init(void);

typedef enum {
    WRITE_POLICY_ACCEPT,
    WRITE_POLICY_REJECT,
} write_policy_t;

typedef struct {
    bool host_active;
    bool configured;
    bool armed;
    bool notify_subscribed;
    bool notify_pending;
    bool disconnect_pending;
    write_policy_t write_policy;
    uint16_t active_connection;
    uint16_t pending_notify_length;
    uint32_t last_host_sequence;
    uint32_t pending_notify_sequence;
    uint32_t pending_disconnect_sequence;
    int64_t last_host_command_ms;
    uint32_t dropped_output_lines;
} transport_state_t;

static const ble_uuid128_t vesc_service_uuid = BLE_UUID128_INIT(
    0x9e, 0xca, 0xdc, 0x24, 0x0e, 0xe5, 0xa9, 0xe0,
    0x93, 0xf3, 0xa3, 0xb5, 0x01, 0x00, 0x40, 0x6e);
static const ble_uuid128_t vesc_write_uuid = BLE_UUID128_INIT(
    0x9e, 0xca, 0xdc, 0x24, 0x0e, 0xe5, 0xa9, 0xe0,
    0x93, 0xf3, 0xa3, 0xb5, 0x02, 0x00, 0x40, 0x6e);
static const ble_uuid128_t vesc_notify_uuid = BLE_UUID128_INIT(
    0x9e, 0xca, 0xdc, 0x24, 0x0e, 0xe5, 0xa9, 0xe0,
    0x93, 0xf3, 0xa3, 0xb5, 0x03, 0x00, 0x40, 0x6e);

static SemaphoreHandle_t output_mutex;
static SemaphoreHandle_t state_mutex;
static transport_state_t state = {
    .active_connection = BLE_HS_CONN_HANDLE_NONE,
    .write_policy = WRITE_POLICY_ACCEPT,
};
static uint8_t own_address_type;
static uint16_t notify_value_handle;
static uint32_t device_sequence;

static int64_t uptime_ms(void) {
    return esp_timer_get_time() / 1000;
}

static void record_output_drop(void) {
    if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(20)) == pdTRUE) {
        state.dropped_output_lines++;
        xSemaphoreGive(state_mutex);
    }
}

static void emit_event(const char *kind, const char *fields) {
    if (xSemaphoreTake(output_mutex, pdMS_TO_TICKS(100)) != pdTRUE) {
        record_output_drop();
        return;
    }

    char line[MAX_OUTPUT_LINE_BYTES];
    const uint32_t sequence = ++device_sequence;
    int used = snprintf(
        line, sizeof(line),
        "{\"protocolVersion\":%d,\"sequence\":%" PRIu32
        ",\"kind\":\"%s\",\"uptimeMs\":%" PRId64,
        CONTROL_PROTOCOL_VERSION, sequence, kind, uptime_ms());
    if (used < 0 || (size_t)used >= sizeof(line)) {
        xSemaphoreGive(output_mutex);
        record_output_drop();
        return;
    }
    if (fields != NULL && fields[0] != '\0') {
        const int appended = snprintf(
            line + used, sizeof(line) - (size_t)used, ",%s", fields);
        if (appended < 0 || (size_t)appended >= sizeof(line) - (size_t)used) {
            xSemaphoreGive(output_mutex);
            record_output_drop();
            return;
        }
        used += appended;
    }
    if ((size_t)used + 2 > sizeof(line)) {
        xSemaphoreGive(output_mutex);
        record_output_drop();
        return;
    }
    line[used++] = '}';
    line[used++] = '\n';

    const int written = usb_serial_jtag_write_bytes(
        line, used, pdMS_TO_TICKS(USB_IO_TIMEOUT_MS));
    xSemaphoreGive(output_mutex);
    if (written != used) {
        record_output_drop();
    }
}

static void emit_command_result(
    uint32_t command_sequence, const char *command_kind, const char *fields) {
    char payload[384];
    const int used = snprintf(
        payload, sizeof(payload),
        "\"commandSequence\":%" PRIu32
        ",\"commandKind\":\"%s\",\"status\":\"ok\"%s%s",
        command_sequence, command_kind,
        fields == NULL || fields[0] == '\0' ? "" : ",",
        fields == NULL ? "" : fields);
    if (used > 0 && (size_t)used < sizeof(payload)) {
        emit_event("commandCompleted", payload);
    }
}

static void emit_command_error(
    uint32_t command_sequence, const char *command_kind,
    const char *code, const char *message) {
    char payload[384];
    const int used = snprintf(
        payload, sizeof(payload),
        "\"commandSequence\":%" PRIu32
        ",\"commandKind\":\"%s\",\"status\":\"error\","
        "\"code\":\"%s\",\"message\":\"%s\"",
        command_sequence, command_kind, code, message);
    if (used > 0 && (size_t)used < sizeof(payload)) {
        emit_event("commandFailed", payload);
    }
}

static int notify_access(
    uint16_t connection_handle, uint16_t attribute_handle,
    struct ble_gatt_access_ctxt *context, void *argument) {
    (void)connection_handle;
    (void)attribute_handle;
    (void)context;
    (void)argument;
    return BLE_ATT_ERR_UNLIKELY;
}

static int write_access(
    uint16_t connection_handle, uint16_t attribute_handle,
    struct ble_gatt_access_ctxt *context, void *argument) {
    (void)attribute_handle;
    (void)argument;

    if (context->op != BLE_GATT_ACCESS_OP_WRITE_CHR) {
        return BLE_ATT_ERR_UNLIKELY;
    }

    const uint16_t length = OS_MBUF_PKTLEN(context->om);
    if (length == 0 || length > MAX_GATT_WRITE_BYTES) {
        return BLE_ATT_ERR_INVALID_ATTR_VALUE_LEN;
    }

    uint8_t bytes[MAX_GATT_WRITE_BYTES];
    uint16_t copied = 0;
    if (ble_hs_mbuf_to_flat(context->om, bytes, sizeof(bytes), &copied) != 0 ||
        copied != length) {
        return BLE_ATT_ERR_UNLIKELY;
    }

    bool accepted = false;
    if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(50)) == pdTRUE) {
        accepted = state.write_policy == WRITE_POLICY_ACCEPT;
        xSemaphoreGive(state_mutex);
    }

    char hex[MAX_GATT_WRITE_BYTES * 2 + 1];
    for (uint16_t index = 0; index < copied; ++index) {
        snprintf(&hex[index * 2], 3, "%02x", bytes[index]);
    }
    char fields[MAX_GATT_WRITE_BYTES * 2 + 128];
    snprintf(
        fields, sizeof(fields),
        "\"connectionHandle\":%u,\"length\":%u,\"hex\":\"%s\","
        "\"accepted\":%s",
        connection_handle, copied, hex, accepted ? "true" : "false");
    emit_event("gattWrite", fields);

    return accepted ? 0 : BLE_ATT_ERR_WRITE_NOT_PERMITTED;
}

static const struct ble_gatt_svc_def services[] = {
    {
        .type = BLE_GATT_SVC_TYPE_PRIMARY,
        .uuid = &vesc_service_uuid.u,
        .characteristics = (struct ble_gatt_chr_def[]) {
            {
                .uuid = &vesc_write_uuid.u,
                .access_cb = write_access,
                .flags = BLE_GATT_CHR_F_WRITE | BLE_GATT_CHR_F_WRITE_NO_RSP,
            },
            {
                .uuid = &vesc_notify_uuid.u,
                .access_cb = notify_access,
                .flags = BLE_GATT_CHR_F_NOTIFY,
                .val_handle = &notify_value_handle,
            },
            {0},
        },
    },
    {0},
};

static int gap_event(struct ble_gap_event *event, void *argument);

static void emit_advertising_changed(bool advertising, const char *reason) {
    char fields[192];
    snprintf(
        fields, sizeof(fields),
        "\"advertising\":%s,\"reason\":\"%s\","
        "\"serviceUuid\":\"6e400001-b5a3-f393-e0a9-e50e24dcca9e\"",
        advertising ? "true" : "false", reason);
    emit_event("advertisingChanged", fields);
}

static int stop_advertising(const char *reason) {
    if (!ble_gap_adv_active()) {
        return 0;
    }
    const int result = ble_gap_adv_stop();
    if (result == 0 || result == BLE_HS_EALREADY) {
        emit_advertising_changed(false, reason);
        return 0;
    }
    return result;
}

static int start_advertising(void) {
    if (ble_gap_adv_active()) {
        return BLE_HS_EALREADY;
    }

    struct ble_hs_adv_fields advertising = {0};
    advertising.flags = BLE_HS_ADV_F_DISC_GEN | BLE_HS_ADV_F_BREDR_UNSUP;
    advertising.uuids128 = (ble_uuid128_t *)&vesc_service_uuid;
    advertising.num_uuids128 = 1;
    advertising.uuids128_is_complete = 1;

    int result = ble_gap_adv_set_fields(&advertising);
    if (result != 0) {
        return result;
    }

    static const uint8_t marker[] = {0xff, 0xff, 'F', 'V', 'T', '1'};
    struct ble_hs_adv_fields scan_response = {0};
    scan_response.name = (uint8_t *)DEVICE_NAME;
    scan_response.name_len = strlen(DEVICE_NAME);
    scan_response.name_is_complete = 1;
    scan_response.mfg_data = (uint8_t *)marker;
    scan_response.mfg_data_len = sizeof(marker);
    result = ble_gap_adv_rsp_set_fields(&scan_response);
    if (result != 0) {
        return result;
    }

    struct ble_gap_adv_params parameters = {0};
    parameters.conn_mode = BLE_GAP_CONN_MODE_UND;
    parameters.disc_mode = BLE_GAP_DISC_MODE_GEN;
    parameters.itvl_min = BLE_GAP_ADV_ITVL_MS(100);
    parameters.itvl_max = BLE_GAP_ADV_ITVL_MS(120);
    result = ble_gap_adv_start(
        own_address_type, NULL, BLE_HS_FOREVER, &parameters, gap_event, NULL);
    if (result == 0) {
        emit_advertising_changed(true, "hostCommand");
    }
    return result;
}

static void clear_connection_state(void) {
    state.active_connection = BLE_HS_CONN_HANDLE_NONE;
    state.notify_subscribed = false;
    state.notify_pending = false;
    state.disconnect_pending = false;
}

static void safe_transport_reset(const char *reason, bool host_lost) {
    uint16_t connection = BLE_HS_CONN_HANDLE_NONE;
    if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(100)) == pdTRUE) {
        state.armed = false;
        state.configured = false;
        state.write_policy = WRITE_POLICY_ACCEPT;
        state.notify_pending = false;
        state.disconnect_pending = false;
        if (host_lost) {
            state.host_active = false;
        }
        connection = state.active_connection;
        xSemaphoreGive(state_mutex);
    }
    stop_advertising(reason);
    if (connection != BLE_HS_CONN_HANDLE_NONE) {
        ble_gap_terminate(connection, BLE_ERR_REM_USER_CONN_TERM);
    }
}

static int gap_event(struct ble_gap_event *event, void *argument) {
    (void)argument;
    char fields[256];

    switch (event->type) {
        case BLE_GAP_EVENT_CONNECT:
            if (event->connect.status == 0) {
                if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(100)) == pdTRUE) {
                    state.active_connection = event->connect.conn_handle;
                    state.notify_subscribed = false;
                    xSemaphoreGive(state_mutex);
                }
                emit_advertising_changed(false, "connected");
                snprintf(
                    fields, sizeof(fields),
                    "\"connectionHandle\":%u", event->connect.conn_handle);
                emit_event("connected", fields);
            } else {
                snprintf(
                    fields, sizeof(fields),
                    "\"stage\":\"connect\",\"status\":%d",
                    event->connect.status);
                emit_event("fault", fields);
            }
            return 0;

        case BLE_GAP_EVENT_DISCONNECT: {
            uint32_t pending_sequence = 0;
            bool restart = false;
            if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(100)) == pdTRUE) {
                if (state.disconnect_pending) {
                    pending_sequence = state.pending_disconnect_sequence;
                }
                clear_connection_state();
                restart = state.armed && state.host_active && state.configured;
                xSemaphoreGive(state_mutex);
            }
            snprintf(
                fields, sizeof(fields),
                "\"reason\":%d", event->disconnect.reason);
            emit_event("disconnected", fields);
            if (pending_sequence != 0) {
                emit_command_result(pending_sequence, "disconnect", NULL);
            }
            if (restart) {
                const int result = start_advertising();
                if (result != 0 && result != BLE_HS_EALREADY) {
                    snprintf(
                        fields, sizeof(fields),
                        "\"stage\":\"restartAdvertising\",\"status\":%d",
                        result);
                    emit_event("fault", fields);
                }
            }
            return 0;
        }

        case BLE_GAP_EVENT_SUBSCRIBE:
            if (event->subscribe.attr_handle == notify_value_handle) {
                if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(100)) == pdTRUE) {
                    state.notify_subscribed = event->subscribe.cur_notify;
                    xSemaphoreGive(state_mutex);
                }
                snprintf(
                    fields, sizeof(fields),
                    "\"connectionHandle\":%u,\"notify\":%s",
                    event->subscribe.conn_handle,
                    event->subscribe.cur_notify ? "true" : "false");
                emit_event("subscriptionChanged", fields);
            }
            return 0;

        case BLE_GAP_EVENT_MTU:
            snprintf(
                fields, sizeof(fields),
                "\"connectionHandle\":%u,\"mtu\":%u",
                event->mtu.conn_handle, event->mtu.value);
            emit_event("mtuChanged", fields);
            return 0;

        case BLE_GAP_EVENT_NOTIFY_TX: {
            uint32_t command_sequence = 0;
            uint16_t length = 0;
            if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(100)) == pdTRUE) {
                if (state.notify_pending) {
                    command_sequence = state.pending_notify_sequence;
                    length = state.pending_notify_length;
                    state.notify_pending = false;
                }
                xSemaphoreGive(state_mutex);
            }
            snprintf(
                fields, sizeof(fields),
                "\"connectionHandle\":%u,\"length\":%u,\"status\":%d",
                event->notify_tx.conn_handle, length, event->notify_tx.status);
            emit_event("notificationSent", fields);
            if (command_sequence != 0) {
                if (event->notify_tx.status == 0) {
                    emit_command_result(
                        command_sequence, "notify",
                        "\"delivery\":\"transmitted\"");
                } else {
                    emit_command_error(
                        command_sequence, "notify", "notificationFailed",
                        "NimBLE did not transmit the notification");
                }
            }
            return 0;
        }

        case BLE_GAP_EVENT_ADV_COMPLETE:
            emit_advertising_changed(false, "stackComplete");
            return 0;

        default:
            return 0;
    }
}

static void stack_reset(int reason) {
    char fields[80];
    snprintf(
        fields, sizeof(fields),
        "\"stage\":\"stackReset\",\"status\":%d", reason);
    emit_event("fault", fields);
}

static void stack_sync(void) {
    const int address_result = ble_hs_util_ensure_addr(0);
    if (address_result != 0 ||
        ble_hs_id_infer_auto(0, &own_address_type) != 0) {
        emit_event("fault", "\"stage\":\"address\"");
        return;
    }
    emit_event("bleReady", "\"advertising\":false");
}

static void emit_state_result(uint32_t command_sequence) {
    bool host_active;
    bool configured;
    bool armed;
    bool subscribed;
    bool connected;
    bool notify_pending;
    uint32_t dropped;
    const char *policy;
    if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(100)) != pdTRUE) {
        emit_command_error(
            command_sequence, "getState", "stateBusy",
            "Transport state is unavailable");
        return;
    }
    host_active = state.host_active;
    configured = state.configured;
    armed = state.armed;
    subscribed = state.notify_subscribed;
    connected = state.active_connection != BLE_HS_CONN_HANDLE_NONE;
    notify_pending = state.notify_pending;
    dropped = state.dropped_output_lines;
    policy = state.write_policy == WRITE_POLICY_ACCEPT ? "accept" : "reject";
    xSemaphoreGive(state_mutex);

    char fields[384];
    snprintf(
        fields, sizeof(fields),
        "\"hostActive\":%s,\"configured\":%s,\"armed\":%s,"
        "\"advertising\":%s,\"connected\":%s,\"subscribed\":%s,"
        "\"notifyPending\":%s,\"writePolicy\":\"%s\","
        "\"maxWriteBytes\":%d,\"maxNotifyBytes\":%d,"
        "\"droppedOutputLines\":%" PRIu32,
        host_active ? "true" : "false",
        configured ? "true" : "false",
        armed ? "true" : "false",
        ble_gap_adv_active() ? "true" : "false",
        connected ? "true" : "false",
        subscribed ? "true" : "false",
        notify_pending ? "true" : "false",
        policy, MAX_GATT_WRITE_BYTES, MAX_NOTIFY_BYTES, dropped);
    emit_command_result(command_sequence, "getState", fields);
}

static void process_command(const char *line) {
    uint32_t protocol_version;
    uint32_t command_sequence;
    char kind[40];
    if (!control_json_u32(line, "protocolVersion", &protocol_version) ||
        !control_json_u32(line, "sequence", &command_sequence) ||
        !control_json_string(line, "kind", kind, sizeof(kind))) {
        emit_event(
            "fault",
            "\"stage\":\"control\",\"code\":\"invalidJson\","
            "\"message\":\"Command must be one flat JSON object\"");
        return;
    }
    if (protocol_version != CONTROL_PROTOCOL_VERSION) {
        emit_command_error(
            command_sequence, kind, "incompatibleProtocol",
            "Expected control protocol version 1");
        return;
    }

    bool stale = false;
    bool host_active = false;
    if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(100)) == pdTRUE) {
        host_active = state.host_active;
        const bool establishing_session =
            strcmp(kind, "hello") == 0 && !host_active;
        stale = !establishing_session &&
            command_sequence <= state.last_host_sequence;
        if (!stale) {
            state.last_host_sequence = command_sequence;
            state.last_host_command_ms = uptime_ms();
        }
        xSemaphoreGive(state_mutex);
    }
    if (stale) {
        emit_command_error(
            command_sequence, kind, "staleSequence",
            "Command sequence must increase monotonically");
        return;
    }

    if (strcmp(kind, "hello") == 0) {
        if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(100)) == pdTRUE) {
            state.host_active = true;
            state.last_host_command_ms = uptime_ms();
            xSemaphoreGive(state_mutex);
        }
        emit_command_result(
            command_sequence, kind,
            "\"firmware\":\"t1\",\"profile\":\"canonicalVescUart\","
            "\"watchdogMs\":5000,\"maxNotifyBytes\":244");
    } else if (!host_active) {
        emit_command_error(
            command_sequence, kind, "helloRequired",
            "hello must establish the host session first");
    } else if (strcmp(kind, "reset") == 0) {
        safe_transport_reset("hostReset", false);
        emit_command_result(command_sequence, kind, NULL);
    } else if (strcmp(kind, "configureGattProfile") == 0) {
        char profile[40];
        if (!control_json_string(line, "profile", profile, sizeof(profile)) ||
            strcmp(profile, PROFILE_NAME) != 0) {
            emit_command_error(
                command_sequence, kind, "unsupportedProfile",
                "Only canonicalVescUart is supported");
        } else {
            if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(100)) == pdTRUE) {
                state.configured = true;
                xSemaphoreGive(state_mutex);
            }
            emit_event(
                "configured",
                "\"profile\":\"canonicalVescUart\","
                "\"name\":\"FloatView Test VESC\","
                "\"manufacturerMarkerHex\":\"ffff46565431\"");
            emit_command_result(command_sequence, kind, NULL);
        }
    } else if (strcmp(kind, "startAdvertising") == 0) {
        bool configured = false;
        bool connected = false;
        if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(100)) == pdTRUE) {
            configured = state.configured;
            connected = state.active_connection != BLE_HS_CONN_HANDLE_NONE;
            xSemaphoreGive(state_mutex);
        }
        if (!configured) {
            emit_command_error(
                command_sequence, kind, "notConfigured",
                "Configure the canonical profile before advertising");
        } else if (connected) {
            emit_command_error(
                command_sequence, kind, "alreadyConnected",
                "Advertising is unavailable while connected");
        } else {
            const int result = start_advertising();
            if (result == 0 || result == BLE_HS_EALREADY) {
                if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(100)) == pdTRUE) {
                    state.armed = true;
                    xSemaphoreGive(state_mutex);
                }
                emit_command_result(command_sequence, kind, NULL);
            } else {
                emit_command_error(
                    command_sequence, kind, "advertisingFailed",
                    "NimBLE could not start advertising");
            }
        }
    } else if (strcmp(kind, "stopAdvertising") == 0) {
        const int result = stop_advertising("hostCommand");
        if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(100)) == pdTRUE) {
            state.armed = false;
            xSemaphoreGive(state_mutex);
        }
        if (result == 0) {
            emit_command_result(command_sequence, kind, NULL);
        } else {
            emit_command_error(
                command_sequence, kind, "stopAdvertisingFailed",
                "NimBLE could not stop advertising");
        }
    } else if (strcmp(kind, "setWritePolicy") == 0) {
        char policy[16];
        if (!control_json_string(line, "policy", policy, sizeof(policy)) ||
            (strcmp(policy, "accept") != 0 && strcmp(policy, "reject") != 0)) {
            emit_command_error(
                command_sequence, kind, "invalidWritePolicy",
                "policy must be accept or reject");
        } else {
            if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(100)) == pdTRUE) {
                state.write_policy = strcmp(policy, "accept") == 0
                    ? WRITE_POLICY_ACCEPT : WRITE_POLICY_REJECT;
                xSemaphoreGive(state_mutex);
            }
            char fields[48];
            snprintf(fields, sizeof(fields), "\"policy\":\"%s\"", policy);
            emit_command_result(command_sequence, kind, fields);
        }
    } else if (strcmp(kind, "notify") == 0) {
        char hex[MAX_NOTIFY_BYTES * 2 + 1];
        uint8_t bytes[MAX_NOTIFY_BYTES];
        size_t length = 0;
        if (!control_json_string(line, "hex", hex, sizeof(hex)) ||
            !control_decode_hex(hex, bytes, sizeof(bytes), &length)) {
            emit_command_error(
                command_sequence, kind, "invalidNotification",
                "hex must contain 1 to 244 bytes");
        } else {
            uint16_t connection = BLE_HS_CONN_HANDLE_NONE;
            bool subscribed = false;
            bool pending = false;
            if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(100)) == pdTRUE) {
                connection = state.active_connection;
                subscribed = state.notify_subscribed;
                pending = state.notify_pending;
                if (connection != BLE_HS_CONN_HANDLE_NONE &&
                    subscribed && !pending) {
                    state.notify_pending = true;
                    state.pending_notify_sequence = command_sequence;
                    state.pending_notify_length = length;
                }
                xSemaphoreGive(state_mutex);
            }
            if (connection == BLE_HS_CONN_HANDLE_NONE) {
                emit_command_error(
                    command_sequence, kind, "notConnected",
                    "A BLE central must be connected");
            } else if (!subscribed) {
                emit_command_error(
                    command_sequence, kind, "notSubscribed",
                    "The central has not enabled notifications");
            } else if (pending) {
                emit_command_error(
                    command_sequence, kind, "notificationBusy",
                    "Only one notification may be pending");
            } else {
                struct os_mbuf *notification =
                    ble_hs_mbuf_from_flat(bytes, length);
                const int result = notification == NULL
                    ? BLE_HS_ENOMEM
                    : ble_gatts_notify_custom(
                        connection, notify_value_handle, notification);
                if (result != 0) {
                    if (xSemaphoreTake(
                            state_mutex, pdMS_TO_TICKS(100)) == pdTRUE) {
                        state.notify_pending = false;
                        xSemaphoreGive(state_mutex);
                    }
                    emit_command_error(
                        command_sequence, kind, "notificationFailed",
                        "NimBLE rejected the notification");
                }
            }
        }
    } else if (strcmp(kind, "disconnect") == 0) {
        uint16_t connection = BLE_HS_CONN_HANDLE_NONE;
        bool pending = false;
        if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(100)) == pdTRUE) {
            connection = state.active_connection;
            pending = state.disconnect_pending;
            if (connection != BLE_HS_CONN_HANDLE_NONE && !pending) {
                state.disconnect_pending = true;
                state.pending_disconnect_sequence = command_sequence;
            }
            xSemaphoreGive(state_mutex);
        }
        if (connection == BLE_HS_CONN_HANDLE_NONE) {
            emit_command_error(
                command_sequence, kind, "notConnected",
                "No BLE central is connected");
        } else if (pending) {
            emit_command_error(
                command_sequence, kind, "disconnectBusy",
                "A disconnect is already pending");
        } else if (ble_gap_terminate(
                       connection, BLE_ERR_REM_USER_CONN_TERM) != 0) {
            if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(100)) == pdTRUE) {
                state.disconnect_pending = false;
                xSemaphoreGive(state_mutex);
            }
            emit_command_error(
                command_sequence, kind, "disconnectFailed",
                "NimBLE rejected the disconnect");
        }
    } else if (strcmp(kind, "getState") == 0) {
        emit_state_result(command_sequence);
    } else {
        emit_command_error(
            command_sequence, kind, "unsupportedCommand",
            "Command kind is not supported by T1");
    }

}

static void nimble_task(void *argument) {
    (void)argument;
    nimble_port_run();
    vTaskDelete(NULL);
}

static void control_task(void *argument) {
    (void)argument;
    char line[MAX_CONTROL_LINE_BYTES + 1];
    size_t used = 0;
    bool discarding = false;

    while (true) {
        uint8_t bytes[64];
        const int count = usb_serial_jtag_read_bytes(
            bytes, sizeof(bytes), pdMS_TO_TICKS(USB_IO_TIMEOUT_MS));
        if (count <= 0) {
            continue;
        }
        for (int index = 0; index < count; ++index) {
            const char character = bytes[index];
            if (character == '\r') {
                continue;
            }
            if (character != '\n') {
                if (!discarding && used < MAX_CONTROL_LINE_BYTES) {
                    line[used++] = character;
                } else {
                    discarding = true;
                }
                continue;
            }
            if (discarding) {
                emit_event(
                    "fault",
                    "\"stage\":\"control\",\"code\":\"lineTooLong\","
                    "\"message\":\"Control line exceeded 768 bytes\"");
            } else if (used > 0) {
                line[used] = '\0';
                process_command(line);
            }
            used = 0;
            discarding = false;
        }
    }
}

static void watchdog_task(void *argument) {
    (void)argument;
    while (true) {
        vTaskDelay(pdMS_TO_TICKS(250));
        bool expired = false;
        if (xSemaphoreTake(state_mutex, pdMS_TO_TICKS(50)) == pdTRUE) {
            expired = state.host_active &&
                uptime_ms() - state.last_host_command_ms > HOST_WATCHDOG_MS;
            xSemaphoreGive(state_mutex);
        }
        if (expired) {
            emit_event(
                "fault",
                "\"stage\":\"hostWatchdog\",\"code\":\"hostTimeout\","
                "\"message\":\"Host command heartbeat expired\"");
            safe_transport_reset("hostTimeout", true);
        }
    }
}

void app_main(void) {
    usb_serial_jtag_driver_config_t usb_config = {
        .tx_buffer_size = 4096,
        .rx_buffer_size = 2048,
    };
    ESP_ERROR_CHECK(usb_serial_jtag_driver_install(&usb_config));

    output_mutex = xSemaphoreCreateMutex();
    state_mutex = xSemaphoreCreateMutex();
    if (output_mutex == NULL || state_mutex == NULL) {
        return;
    }

    emit_event(
        "booted",
        "\"firmware\":\"t1\",\"target\":\"esp32s3\","
        "\"advertising\":false");

    esp_err_t result = nvs_flash_init();
    if (result == ESP_ERR_NVS_NO_FREE_PAGES ||
        result == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_ERROR_CHECK(nvs_flash_erase());
        result = nvs_flash_init();
    }
    ESP_ERROR_CHECK(result);

    ESP_ERROR_CHECK(nimble_port_init());
    ble_svc_gap_init();
    ble_svc_gatt_init();
    ESP_ERROR_CHECK(ble_svc_gap_device_name_set(DEVICE_NAME));
    ESP_ERROR_CHECK(ble_att_set_preferred_mtu(247));
    ESP_ERROR_CHECK(ble_gatts_count_cfg(services));
    ESP_ERROR_CHECK(ble_gatts_add_svcs(services));

    ble_hs_cfg.reset_cb = stack_reset;
    ble_hs_cfg.sync_cb = stack_sync;
    ble_hs_cfg.store_status_cb = ble_store_util_status_rr;
    ble_store_config_init();

    xTaskCreate(nimble_task, "nimble", 4096, NULL, 5, NULL);
    xTaskCreate(control_task, "control", 6144, NULL, 5, NULL);
    xTaskCreate(watchdog_task, "watchdog", 3072, NULL, 4, NULL);
}
