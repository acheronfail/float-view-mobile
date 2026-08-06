package fail.acheron.floatview.protocol.vesc

enum class VescCommand(val id: Int) {
    GET_FIRMWARE_VERSION(0),
    GET_REALTIME_VALUES(4),
    GET_MOTOR_CONFIGURATION(14),
    FORWARD_CAN(34),
    CUSTOM_APP_DATA(36),
    PING_CAN(62),
    BMS_GET_VALUES(96),
    SHUTDOWN(156),
    ;

    val byte: Byte
        get() = id.toByte()
}
