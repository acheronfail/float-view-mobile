#pragma once

#include "esp_err.h"

typedef enum {
    STATUS_LED_READY,
    STATUS_LED_ADVERTISING,
    STATUS_LED_CONNECTED,
} status_led_state_t;

esp_err_t status_led_init(void);
esp_err_t status_led_set(status_led_state_t state);
