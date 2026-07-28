#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "control_protocol.h"
#include "driver/usb_serial_jtag.h"
#include "esp_err.h"
#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"
#include "freertos/task.h"
#include "host/ble_att.h"
#include "host/ble_gap.h"
#include "host/ble_gatt.h"
#include "host/ble_hs.h"
#include "host/ble_uuid.h"
#include "host/util/util.h"
#include "nimble/nimble_port.h"
#include "nvs_flash.h"
#include "os/os_mbuf.h"
#include "services/gap/ble_svc_gap.h"
#include "services/gatt/ble_svc_gatt.h"
#include "status_led.h"

#define DEVICE_NAME "FloatView Test VESC"
#define MAX_GATT_WRITE_BYTES 256
#define USB_TIMEOUT_MS 20

void ble_store_config_init(void);

static const ble_uuid128_t service_uuid = BLE_UUID128_INIT(
    0x9e, 0xca, 0xdc, 0x24, 0x0e, 0xe5, 0xa9, 0xe0,
    0x93, 0xf3, 0xa3, 0xb5, 0x01, 0x00, 0x40, 0x6e);
static const ble_uuid128_t write_uuid = BLE_UUID128_INIT(
    0x9e, 0xca, 0xdc, 0x24, 0x0e, 0xe5, 0xa9, 0xe0,
    0x93, 0xf3, 0xa3, 0xb5, 0x02, 0x00, 0x40, 0x6e);
static const ble_uuid128_t notify_uuid = BLE_UUID128_INIT(
    0x9e, 0xca, 0xdc, 0x24, 0x0e, 0xe5, 0xa9, 0xe0,
    0x93, 0xf3, 0xa3, 0xb5, 0x03, 0x00, 0x40, 0x6e);

static SemaphoreHandle_t output_mutex;
static SemaphoreHandle_t state_mutex;
static uint16_t connection_handle = BLE_HS_CONN_HANDLE_NONE;
static uint16_t notify_handle;
static bool subscribed;
static uint8_t address_type;

static void emit(const char *line) {
    if (xSemaphoreTake(output_mutex, pdMS_TO_TICKS(100)) != pdTRUE) return;
    usb_serial_jtag_write_bytes(line, strlen(line), pdMS_TO_TICKS(USB_TIMEOUT_MS));
    usb_serial_jtag_write_bytes("\n", 1, pdMS_TO_TICKS(USB_TIMEOUT_MS));
    xSemaphoreGive(output_mutex);
}

static void error(const char *code) {
    char line[64];
    snprintf(line, sizeof(line), "ERROR %s", code);
    emit(line);
}

static int notify_access(
    uint16_t connection, uint16_t attribute,
    struct ble_gatt_access_ctxt *context, void *argument) {
    (void)connection;
    (void)attribute;
    (void)context;
    (void)argument;
    return BLE_ATT_ERR_UNLIKELY;
}

static int write_access(
    uint16_t connection, uint16_t attribute,
    struct ble_gatt_access_ctxt *context, void *argument) {
    (void)connection;
    (void)attribute;
    (void)argument;
    if (context->op != BLE_GATT_ACCESS_OP_WRITE_CHR) return BLE_ATT_ERR_UNLIKELY;
    const uint16_t length = OS_MBUF_PKTLEN(context->om);
    if (length == 0 || length > MAX_GATT_WRITE_BYTES) {
        error("WRITE_TOO_LONG");
        return BLE_ATT_ERR_INVALID_ATTR_VALUE_LEN;
    }
    uint8_t bytes[MAX_GATT_WRITE_BYTES];
    uint16_t copied = 0;
    if (ble_hs_mbuf_to_flat(context->om, bytes, sizeof(bytes), &copied) != 0 ||
        copied != length) {
        error("WRITE_FAILED");
        return BLE_ATT_ERR_UNLIKELY;
    }
    static const char hex[] = "0123456789ABCDEF";
    char line[6 + MAX_GATT_WRITE_BYTES * 2 + 1] = "WRITE ";
    for (uint16_t index = 0; index < copied; index++) {
        line[6 + index * 2] = hex[bytes[index] >> 4];
        line[7 + index * 2] = hex[bytes[index] & 0x0f];
    }
    line[6 + copied * 2] = '\0';
    emit(line);
    return 0;
}

static const struct ble_gatt_svc_def services[] = {
    {
        .type = BLE_GATT_SVC_TYPE_PRIMARY,
        .uuid = &service_uuid.u,
        .characteristics = (struct ble_gatt_chr_def[]) {
            {
                .uuid = &write_uuid.u,
                .access_cb = write_access,
                .flags = BLE_GATT_CHR_F_WRITE | BLE_GATT_CHR_F_WRITE_NO_RSP,
            },
            {
                .uuid = &notify_uuid.u,
                .access_cb = notify_access,
                .flags = BLE_GATT_CHR_F_NOTIFY,
                .val_handle = &notify_handle,
            },
            {0},
        },
    },
    {0},
};

static int gap_event(struct ble_gap_event *event, void *argument) {
    (void)argument;
    switch (event->type) {
        case BLE_GAP_EVENT_CONNECT:
            if (event->connect.status != 0) {
                error("CONNECT_FAILED");
                return 0;
            }
            xSemaphoreTake(state_mutex, portMAX_DELAY);
            connection_handle = event->connect.conn_handle;
            subscribed = false;
            xSemaphoreGive(state_mutex);
            emit("CONNECTED");
            return 0;
        case BLE_GAP_EVENT_DISCONNECT:
            xSemaphoreTake(state_mutex, portMAX_DELAY);
            connection_handle = BLE_HS_CONN_HANDLE_NONE;
            subscribed = false;
            xSemaphoreGive(state_mutex);
            emit("DISCONNECTED");
            return 0;
        case BLE_GAP_EVENT_SUBSCRIBE:
            if (event->subscribe.attr_handle == notify_handle) {
                xSemaphoreTake(state_mutex, portMAX_DELAY);
                subscribed = event->subscribe.cur_notify;
                xSemaphoreGive(state_mutex);
            }
            return 0;
        default:
            return 0;
    }
}

static int advertise(void) {
    if (ble_gap_adv_active()) return 0;
    struct ble_hs_adv_fields fields = {0};
    fields.flags = BLE_HS_ADV_F_DISC_GEN | BLE_HS_ADV_F_BREDR_UNSUP;
    fields.uuids128 = (ble_uuid128_t *)&service_uuid;
    fields.num_uuids128 = 1;
    fields.uuids128_is_complete = 1;
    int result = ble_gap_adv_set_fields(&fields);
    if (result != 0) return result;

    struct ble_hs_adv_fields response = {0};
    response.name = (uint8_t *)DEVICE_NAME;
    response.name_len = strlen(DEVICE_NAME);
    response.name_is_complete = 1;
    result = ble_gap_adv_rsp_set_fields(&response);
    if (result != 0) return result;

    struct ble_gap_adv_params parameters = {0};
    parameters.conn_mode = BLE_GAP_CONN_MODE_UND;
    parameters.disc_mode = BLE_GAP_DISC_MODE_GEN;
    parameters.itvl_min = BLE_GAP_ADV_ITVL_MS(100);
    parameters.itvl_max = BLE_GAP_ADV_ITVL_MS(120);
    return ble_gap_adv_start(
        address_type, NULL, BLE_HS_FOREVER, &parameters, gap_event, NULL);
}

static void reset_transport(void) {
    if (ble_gap_adv_active()) ble_gap_adv_stop();
    xSemaphoreTake(state_mutex, portMAX_DELAY);
    const uint16_t connection = connection_handle;
    xSemaphoreGive(state_mutex);
    if (connection != BLE_HS_CONN_HANDLE_NONE) {
        ble_gap_terminate(connection, BLE_ERR_REM_USER_CONN_TERM);
    }
    status_led_rgb(0, 0, 0);
}

static void process(const char *line) {
    control_command_t command;
    if (!control_parse(line, &command)) {
        error("INVALID_COMMAND");
        return;
    }
    switch (command.kind) {
        case CONTROL_RESET:
            reset_transport();
            emit("READY");
            return;
        case CONTROL_ADVERTISE:
            if (advertise() != 0) error("ADVERTISE_FAILED");
            return;
        case CONTROL_DISCONNECT: {
            xSemaphoreTake(state_mutex, portMAX_DELAY);
            const uint16_t connection = connection_handle;
            xSemaphoreGive(state_mutex);
            if (connection == BLE_HS_CONN_HANDLE_NONE) error("NOT_CONNECTED");
            else if (ble_gap_terminate(connection, BLE_ERR_REM_USER_CONN_TERM) != 0) {
                error("DISCONNECT_FAILED");
            }
            return;
        }
        case CONTROL_NOTIFY: {
            xSemaphoreTake(state_mutex, portMAX_DELAY);
            const uint16_t connection = connection_handle;
            const bool can_notify = subscribed;
            xSemaphoreGive(state_mutex);
            if (connection == BLE_HS_CONN_HANDLE_NONE) {
                error("NOT_CONNECTED");
                return;
            }
            if (!can_notify) {
                error("NOT_SUBSCRIBED");
                return;
            }
            struct os_mbuf *buffer =
                ble_hs_mbuf_from_flat(command.bytes, command.length);
            if (buffer == NULL ||
                ble_gatts_notify_custom(connection, notify_handle, buffer) != 0) {
                error("NOTIFY_FAILED");
            }
            return;
        }
        case CONTROL_LED:
            if (status_led_rgb(command.red, command.green, command.blue) != ESP_OK) {
                error("LED_FAILED");
            }
            return;
    }
}

static void stack_reset(int reason) {
    (void)reason;
    error("BLE_RESET");
}

static void stack_sync(void) {
    if (ble_hs_util_ensure_addr(0) != 0 ||
        ble_hs_id_infer_auto(0, &address_type) != 0) {
        error("BLE_ADDRESS");
        return;
    }
    emit("READY");
}

static void nimble_task(void *argument) {
    (void)argument;
    nimble_port_run();
    vTaskDelete(NULL);
}

static void control_task(void *argument) {
    (void)argument;
    char line[CONTROL_MAX_LINE_BYTES + 1];
    size_t used = 0;
    bool overflow = false;
    while (true) {
        uint8_t bytes[64];
        const int count = usb_serial_jtag_read_bytes(
            bytes, sizeof(bytes), pdMS_TO_TICKS(USB_TIMEOUT_MS));
        for (int index = 0; index < count; index++) {
            const char value = bytes[index];
            if (value == '\r') continue;
            if (value != '\n') {
                if (used < CONTROL_MAX_LINE_BYTES) line[used++] = value;
                else overflow = true;
                continue;
            }
            if (overflow) error("LINE_TOO_LONG");
            else if (used > 0) {
                line[used] = '\0';
                process(line);
            }
            used = 0;
            overflow = false;
        }
    }
}

void app_main(void) {
    usb_serial_jtag_driver_config_t usb = {
        .tx_buffer_size = 1024,
        .rx_buffer_size = 1024,
    };
    ESP_ERROR_CHECK(usb_serial_jtag_driver_install(&usb));
    output_mutex = xSemaphoreCreateMutex();
    state_mutex = xSemaphoreCreateMutex();
    ESP_ERROR_CHECK(status_led_init());

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
    xTaskCreate(control_task, "control", 4096, NULL, 5, NULL);
}
