idf_component_register(
    SRCS "main.c" "control_protocol.c" "status_led.c"
    PRIV_REQUIRES
        bt
        nvs_flash
        esp_timer
        esp_driver_rmt
        esp_driver_usb_serial_jtag
)
