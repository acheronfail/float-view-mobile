package fail.acheron.floatview.protocol.vesc

import fail.acheron.floatview.protocol.VescFrameCodec
import fail.acheron.floatview.protocol.refloat.RefloatCommand
import fail.acheron.floatview.protocol.refloat.RefloatWire

/** Minimal deterministic direct VESC used only by software-host conformance. */
class SoftwareVescPeripheral {
    fun response(frame: ByteArray): ByteArray? {
        val request = runCatching { VescFrameCodec.decode(frame) }.getOrNull() ?: return null
        val payload =
            when {
                request.contentEquals(byteArrayOf(VescCommand.GET_FIRMWARE_VERSION.byte)) -> firmware
                request.contentEquals(byteArrayOf(VescCommand.PING_CAN.byte)) ->
                    byteArrayOf(VescCommand.PING_CAN.byte, BMS_CAN_ID.toByte())
                request.contentEquals(
                    byteArrayOf(
                        VescCommand.FORWARD_CAN.byte,
                        BMS_CAN_ID.toByte(),
                        VescCommand.GET_FIRMWARE_VERSION.byte,
                    ),
                ) -> bmsFirmware
                request.contentEquals(
                    byteArrayOf(
                        VescCommand.FORWARD_CAN.byte,
                        BMS_CAN_ID.toByte(),
                        VescCommand.BMS_GET_VALUES.byte,
                    ),
                ) -> bmsValues
                request.contentEquals(
                    RefloatWire.request(
                        RefloatCommand.INFO,
                        RefloatWire.INFO_PROTOCOL_VERSION,
                        RefloatWire.INFO_FLAGS,
                    ),
                ) -> RefloatWire.request(RefloatCommand.INFO, 10)
                request.contentEquals(
                    RefloatWire.request(
                        RefloatCommand.GET_ALL_DATA,
                        RefloatWire.LEGACY_TELEMETRY_DETAIL_LEVEL,
                    ),
                ) -> telemetry
                else -> null
            } ?: return null
        return VescFrameCodec.encode(payload)
    }

    private companion object {
        const val BMS_CAN_ID = 42
        val firmware =
            byteArrayOf(VescCommand.GET_FIRMWARE_VERSION.byte, 6, 6) +
                "THOR300".encodeToByteArray() +
                byteArrayOf(0) +
                ByteArray(12) +
                byteArrayOf(0, 0, 0, 0, 0, 0, 0, 0) +
                "VESC".encodeToByteArray() + byteArrayOf(0)
        val telemetry =
            ByteArray(55).apply {
                this[0] = VescCommand.CUSTOM_APP_DATA.byte
                this[1] = RefloatWire.PACKAGE_ID.toByte()
                this[2] = RefloatCommand.GET_ALL_DATA.byte
                this[3] = 3
                this[23] = 2
                this[24] = 4
                // Legacy Refloat 1.0 reads the ride state from this low nibble.
                this[10] = 1
                this[33] = 128.toByte()
                this[39] = 60
                this[40] = 56
            }
        val bmsFirmware = "00060056424d530000112233445566778899aabb00000101".hexBytes()
        val bmsValues =
            "6003010b0003412140ff43a158ff445120000004e20000f424031068105e10540100010209f60a410abe09ab14820b6d036b03b62a428000004480000042000000440000002794014f4b00".hexBytes()

        fun String.hexBytes(): ByteArray = chunked(2).map { it.toInt(16).toByte() }.toByteArray()
    }
}
