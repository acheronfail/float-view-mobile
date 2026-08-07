package fail.acheron.floatview.bluetooth

import java.io.File

/** Bounded emulator-only byte fixture; production protocol behavior lives in Rust. */
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
        val request = SoftwareVescFrame.decode(frame) ?: return null
        val payload = when {
            request.contentEquals(byteArrayOf(GET_FIRMWARE_VERSION)) -> firmware
            request.contentEquals(byteArrayOf(PING_CAN)) -> byteArrayOf(PING_CAN, BMS_CAN_ID)
            request.contentEquals(byteArrayOf(FORWARD_CAN, BMS_CAN_ID, GET_FIRMWARE_VERSION)) ->
                bmsFirmware
            request.contentEquals(byteArrayOf(FORWARD_CAN, BMS_CAN_ID, BMS_GET_VALUES)) -> bmsValues
            request.contentEquals(refloatRequest(INFO, INFO_PROTOCOL_VERSION, INFO_FLAGS)) ->
                refloatRequest(INFO, 10)
            request.contentEquals(refloatRequest(GET_ALL_DATA, LEGACY_TELEMETRY_DETAIL_LEVEL)) ->
                telemetry
            request.firstOrNull()?.toInt()?.and(0xff) in 92..95 -> configurationResponse(request)
            else -> null
        } ?: return null
        return SoftwareVescFrame.encode(payload)
    }

    private fun configurationResponse(request: ByteArray): ByteArray? {
        return when (request.first().toInt() and 0xff) {
            GET_CUSTOM_CONFIGURATION_XML.toInt() -> {
                if (request.size != 10 || request[1].toInt() != 0) return null
                val length = i32(request, 2) ?: return null
                val offset = i32(request, 6) ?: return null
                if (length !in 1..400 || offset !in configurationSchema.indices) return null
                byteArrayOf(GET_CUSTOM_CONFIGURATION_XML, 0) +
                    i32Bytes(configurationSchema.size) + i32Bytes(offset) +
                    configurationSchema.copyOfRange(offset, minOf(configurationSchema.size, offset + length))
            }
            GET_CUSTOM_CONFIGURATION.toInt() -> {
                if (!request.contentEquals(byteArrayOf(GET_CUSTOM_CONFIGURATION, 0))) return null
                byteArrayOf(GET_CUSTOM_CONFIGURATION, 0) + currentConfiguration
            }
            GET_CUSTOM_CONFIGURATION_DEFAULT.toInt() -> {
                if (!request.contentEquals(byteArrayOf(GET_CUSTOM_CONFIGURATION_DEFAULT, 0))) return null
                byteArrayOf(GET_CUSTOM_CONFIGURATION_DEFAULT, 0) + defaultConfiguration
            }
            SET_CUSTOM_CONFIGURATION.toInt() -> {
                if (request.size <= 2 || request[1].toInt() != 0 ||
                    !request.copyOfRange(2, request.size).contentEquals(acceptedConfigurationWrite)
                ) return null
                currentConfiguration = expectedConfigurationReadback.copyOf()
                byteArrayOf(SET_CUSTOM_CONFIGURATION)
            }
            else -> null
        }
    }

    private companion object {
        const val GET_FIRMWARE_VERSION: Byte = 0
        const val FORWARD_CAN: Byte = 34
        const val CUSTOM_APP_DATA: Byte = 36
        const val PING_CAN: Byte = 62
        const val GET_CUSTOM_CONFIGURATION_XML: Byte = 92
        const val GET_CUSTOM_CONFIGURATION: Byte = 93
        const val GET_CUSTOM_CONFIGURATION_DEFAULT: Byte = 94
        const val SET_CUSTOM_CONFIGURATION: Byte = 95
        const val BMS_GET_VALUES: Byte = 96
        const val BMS_CAN_ID: Byte = 42
        const val PACKAGE_ID: Byte = 101
        const val INFO: Byte = 0
        const val GET_ALL_DATA: Byte = 10
        const val INFO_PROTOCOL_VERSION = 2
        const val INFO_FLAGS = 0
        const val LEGACY_TELEMETRY_DETAIL_LEVEL = 4

        fun refloatRequest(command: Byte, vararg parameters: Int) = byteArrayOf(
            CUSTOM_APP_DATA,
            PACKAGE_ID,
            command,
            *parameters.map(Int::toByte).toByteArray(),
        )

        val firmware = byteArrayOf(GET_FIRMWARE_VERSION, 6, 6) +
            "THOR300".encodeToByteArray() + byteArrayOf(0) + ByteArray(12) +
            byteArrayOf(0, 0, 0, 1, 0, 0, 0, 0) + "VESC".encodeToByteArray() + byteArrayOf(0)
        val telemetry = ByteArray(55).apply {
            this[0] = CUSTOM_APP_DATA
            this[1] = PACKAGE_ID
            this[2] = GET_ALL_DATA
            this[3] = 3
            this[23] = 2
            this[24] = 4
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

private object SoftwareVescFrame {
    private const val MAXIMUM_PAYLOAD_BYTES = 4_096

    fun encode(payload: ByteArray): ByteArray {
        if (payload.size > MAXIMUM_PAYLOAD_BYTES) return byteArrayOf()
        val header = if (payload.size <= 255) {
            byteArrayOf(2, payload.size.toByte())
        } else {
            byteArrayOf(3, (payload.size shr 8).toByte(), payload.size.toByte())
        }
        val crc = checksum(payload)
        return header + payload + byteArrayOf((crc shr 8).toByte(), crc.toByte(), 3)
    }

    fun decode(frame: ByteArray): ByteArray? {
        val headerSize: Int
        val payloadSize: Int
        when (frame.firstOrNull()?.toInt()?.and(0xff)) {
            2 -> {
                if (frame.size < 5) return null
                headerSize = 2
                payloadSize = frame[1].toInt() and 0xff
            }
            3 -> {
                if (frame.size < 6) return null
                headerSize = 3
                payloadSize = ((frame[1].toInt() and 0xff) shl 8) or (frame[2].toInt() and 0xff)
            }
            else -> return null
        }
        if (payloadSize > MAXIMUM_PAYLOAD_BYTES || frame.size != headerSize + payloadSize + 3 ||
            frame.last().toInt() != 3
        ) return null
        val payload = frame.copyOfRange(headerSize, headerSize + payloadSize)
        val received = ((frame[headerSize + payloadSize].toInt() and 0xff) shl 8) or
            (frame[headerSize + payloadSize + 1].toInt() and 0xff)
        return payload.takeIf { checksum(it) == received }
    }

    private fun checksum(bytes: ByteArray): Int {
        var crc = 0
        for (byte in bytes) {
            crc = crc xor ((byte.toInt() and 0xff) shl 8)
            repeat(8) {
                crc = if (crc and 0x8000 != 0) (crc shl 1) xor 0x1021 else crc shl 1
                crc = crc and 0xffff
            }
        }
        return crc
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
