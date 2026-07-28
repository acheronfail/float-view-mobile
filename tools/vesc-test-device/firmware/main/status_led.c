#include "status_led.h"

#include <stddef.h>
#include <stdint.h>

#include "driver/rmt_tx.h"
#include "freertos/FreeRTOS.h"

#define STATUS_LED_GPIO 48
#define STATUS_LED_RESOLUTION_HZ 10000000
#define STATUS_LED_DIM_BRIGHTNESS 8
#define STATUS_LED_BRIGHTNESS 32

static rmt_channel_handle_t led_channel;
static rmt_encoder_handle_t led_encoder;

static const rmt_symbol_word_t ws2812_zero = {
    .level0 = 1,
    .duration0 = 3,
    .level1 = 0,
    .duration1 = 9,
};

static const rmt_symbol_word_t ws2812_one = {
    .level0 = 1,
    .duration0 = 9,
    .level1 = 0,
    .duration1 = 3,
};

static const rmt_symbol_word_t ws2812_reset = {
    .level0 = 0,
    .duration0 = 250,
    .level1 = 0,
    .duration1 = 250,
};

static size_t encode_ws2812(
    const void *data, size_t data_size, size_t symbols_written,
    size_t symbols_free, rmt_symbol_word_t *symbols, bool *done, void *argument) {
    (void)argument;
    if (symbols_free < 8) {
        return 0;
    }

    const size_t data_position = symbols_written / 8;
    if (data_position >= data_size) {
        symbols[0] = ws2812_reset;
        *done = true;
        return 1;
    }

    const uint8_t byte = ((const uint8_t *)data)[data_position];
    size_t output_position = 0;
    for (uint8_t mask = 0x80; mask != 0; mask >>= 1) {
        symbols[output_position++] =
            (byte & mask) != 0 ? ws2812_one : ws2812_zero;
    }
    return output_position;
}

esp_err_t status_led_init(void) {
    const rmt_tx_channel_config_t channel_config = {
        .gpio_num = STATUS_LED_GPIO,
        .clk_src = RMT_CLK_SRC_DEFAULT,
        .resolution_hz = STATUS_LED_RESOLUTION_HZ,
        .mem_block_symbols = 64,
        .trans_queue_depth = 1,
    };
    esp_err_t result = rmt_new_tx_channel(&channel_config, &led_channel);
    if (result != ESP_OK) {
        return result;
    }

    const rmt_simple_encoder_config_t encoder_config = {
        .callback = encode_ws2812,
        .min_chunk_size = 8,
    };
    result = rmt_new_simple_encoder(&encoder_config, &led_encoder);
    if (result != ESP_OK) {
        return result;
    }
    result = rmt_enable(led_channel);
    if (result != ESP_OK) {
        return result;
    }
    return status_led_set(STATUS_LED_READY);
}

esp_err_t status_led_set(status_led_state_t state) {
    if (led_channel == NULL || led_encoder == NULL) {
        return ESP_ERR_INVALID_STATE;
    }
    uint8_t pixel_grb[3] = {0};
    switch (state) {
        case STATUS_LED_READY:
            pixel_grb[0] = STATUS_LED_DIM_BRIGHTNESS;
            break;
        case STATUS_LED_ADVERTISING:
            pixel_grb[0] = STATUS_LED_BRIGHTNESS;
            pixel_grb[1] = STATUS_LED_BRIGHTNESS;
            pixel_grb[2] = STATUS_LED_BRIGHTNESS;
            break;
        case STATUS_LED_CONNECTED:
            pixel_grb[2] = STATUS_LED_BRIGHTNESS;
            break;
        default:
            return ESP_ERR_INVALID_ARG;
    }
    const rmt_transmit_config_t transmit_config = {
        .loop_count = 0,
    };
    esp_err_t result = rmt_transmit(
        led_channel, led_encoder, pixel_grb, sizeof(pixel_grb),
        &transmit_config);
    if (result != ESP_OK) {
        return result;
    }
    return rmt_tx_wait_all_done(led_channel, pdMS_TO_TICKS(100));
}
