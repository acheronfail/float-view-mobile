package fail.acheron.floatview.protocol.refloat

import fail.acheron.floatview.protocol.vesc.VescCommand

enum class RefloatCommand(val id: Int) {
    INFO(0),
    GET_ALL_DATA(10),
    PACKAGE_LOCK(12),
    STABLE_LIGHTS(20),
    FLYWHEEL(22),
    REALTIME_DATA_INTERNAL(31),
    REALTIME_DATA_INTERNAL_IDS(32),
    ALERTS_CONTROL(36),
    LEGACY_LIGHTS(202),
    ;

    val byte: Byte
        get() = id.toByte()

    companion object {
        fun fromId(id: Int): RefloatCommand? = entries.firstOrNull { it.id == id }
    }
}

object RefloatWire {
    const val PACKAGE_ID = 101
    const val INFO_PROTOCOL_VERSION = 2
    const val INFO_FLAGS = 0
    const val LEGACY_TELEMETRY_DETAIL_LEVEL = 4

    fun request(command: RefloatCommand, vararg parameters: Int): ByteArray =
        byteArrayOf(
            VescCommand.CUSTOM_APP_DATA.byte,
            PACKAGE_ID.toByte(),
            command.byte,
            *parameters.map(Int::toByte).toByteArray(),
        )

    fun command(payload: ByteArray): RefloatCommand? {
        if (
            payload.size < 3 ||
            payload[0].toInt() and 0xff != VescCommand.CUSTOM_APP_DATA.id ||
            payload[1].toInt() and 0xff != PACKAGE_ID
        ) {
            return null
        }
        return RefloatCommand.fromId(payload[2].toInt() and 0xff)
    }
}
