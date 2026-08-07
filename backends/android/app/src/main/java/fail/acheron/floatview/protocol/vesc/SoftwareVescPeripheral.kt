package fail.acheron.floatview.protocol.vesc

import fail.acheron.floatview.protocol.VescFrameCodec
import fail.acheron.floatview.protocol.refloat.RefloatCommand
import fail.acheron.floatview.protocol.refloat.RefloatWire
import java.io.File

/** Minimal deterministic direct VESC used only by software-host conformance. */
class SoftwareVescPeripheral(
    fixtureLoader: (String) -> ByteArray = ::loadDevelopmentFixture,
) {
    private val configurationSchema = fixtureLoader("schemas/1.0.0.qcompress").bounded(32_768)
    private val defaultConfiguration = fixtureLoader("values/1.0.0-default.bin").bounded(1_024)
    private val acceptedConfigurationWrite = fixtureLoader("values/1.0.0-write.bin").bounded(1_024)
    private val expectedConfigurationReadback = fixtureLoader("values/1.0.0-readback.bin").bounded(1_024)
    private var currentConfiguration = fixtureLoader("values/1.0.0-current.bin").bounded(1_024)

    @Synchronized
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
                request.firstOrNull()?.toInt()?.and(0xff) in 92..95 -> configurationResponse(request)
                else -> null
            } ?: return null
        return VescFrameCodec.encode(payload)
    }

    private fun configurationResponse(request: ByteArray): ByteArray? {
        return when (request.first().toInt() and 0xff) {
        VescCommand.GET_CUSTOM_CONFIGURATION_XML.id -> {
            if (request.size != 10 || request[1].toInt() != 0) return null
            val length = i32(request, 2) ?: return null
            val offset = i32(request, 6) ?: return null
            if (length !in 1..400 || offset !in configurationSchema.indices) return null
            byteArrayOf(VescCommand.GET_CUSTOM_CONFIGURATION_XML.byte, 0) +
                i32Bytes(configurationSchema.size) + i32Bytes(offset) +
                configurationSchema.copyOfRange(offset, minOf(configurationSchema.size, offset + length))
        }
        VescCommand.GET_CUSTOM_CONFIGURATION.id -> {
            if (!request.contentEquals(byteArrayOf(VescCommand.GET_CUSTOM_CONFIGURATION.byte, 0))) return null
            byteArrayOf(VescCommand.GET_CUSTOM_CONFIGURATION.byte, 0) + currentConfiguration
        }
        VescCommand.GET_CUSTOM_CONFIGURATION_DEFAULT.id -> {
            if (!request.contentEquals(byteArrayOf(VescCommand.GET_CUSTOM_CONFIGURATION_DEFAULT.byte, 0))) return null
            byteArrayOf(VescCommand.GET_CUSTOM_CONFIGURATION_DEFAULT.byte, 0) + defaultConfiguration
        }
        VescCommand.SET_CUSTOM_CONFIGURATION.id -> {
            if (request.size <= 2 || request[1].toInt() != 0 ||
                !request.copyOfRange(2, request.size).contentEquals(acceptedConfigurationWrite)
            ) return null
            currentConfiguration = expectedConfigurationReadback.copyOf()
            byteArrayOf(VescCommand.SET_CUSTOM_CONFIGURATION.byte)
        }
            else -> null
        }
    }

    private companion object {
        const val BMS_CAN_ID = 42
        val firmware =
            byteArrayOf(VescCommand.GET_FIRMWARE_VERSION.byte, 6, 6) +
                "THOR300".encodeToByteArray() +
                byteArrayOf(0) +
                ByteArray(12) +
                byteArrayOf(0, 0, 0, 1, 0, 0, 0, 0) +
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

private fun loadDevelopmentFixture(path: String): ByteArray {
    val root = System.getProperty("floatViewFixtureRoot")
        ?: throw IllegalStateException("Software Refloat configuration fixtures are unavailable")
    return File(root, "refloat-configuration/$path").readBytes()
}

private fun ByteArray.bounded(maximum: Int) = also {
    require(isNotEmpty() && size <= maximum) { "Invalid bounded software Refloat configuration fixture" }
}

private fun i32(bytes: ByteArray, offset: Int): Int? {
    if (offset + 4 > bytes.size) return null
    val value = ((bytes[offset].toLong() and 0xff) shl 24) or
        ((bytes[offset + 1].toLong() and 0xff) shl 16) or
        ((bytes[offset + 2].toLong() and 0xff) shl 8) or
        (bytes[offset + 3].toLong() and 0xff)
    return value.takeIf { it <= Int.MAX_VALUE }?.toInt()
}

private fun i32Bytes(value: Int) = byteArrayOf(
    (value ushr 24).toByte(),
    (value ushr 16).toByte(),
    (value ushr 8).toByte(),
    value.toByte(),
)
