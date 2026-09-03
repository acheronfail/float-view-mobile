package fail.acheron.floatview.protocol.vesc

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
        val request = decodeFrame(frame) ?: return null
        val payload = when {
            request.contentEquals(byteArrayOf(GET_FIRMWARE.toByte())) -> firmware
            request.contentEquals(byteArrayOf(PING_CAN.toByte())) -> byteArrayOf(PING_CAN.toByte(), BMS_CAN_ID.toByte())
            request.contentEquals(byteArrayOf(FORWARD_CAN.toByte(), BMS_CAN_ID.toByte(), GET_FIRMWARE.toByte())) ->
                bmsFirmware
            request.contentEquals(byteArrayOf(FORWARD_CAN.toByte(), BMS_CAN_ID.toByte(), BMS_VALUES.toByte())) ->
                bmsValues
            request.contentEquals(byteArrayOf(CUSTOM_APP_DATA.toByte(), PACKAGE_ID.toByte(), INFO, 2, 0)) ->
                byteArrayOf(CUSTOM_APP_DATA.toByte(), PACKAGE_ID.toByte(), INFO, 10)
            request.contentEquals(byteArrayOf(CUSTOM_APP_DATA.toByte(), PACKAGE_ID.toByte(), GET_ALL_DATA, 4)) ->
                telemetry
            request.firstOrNull()?.toInt()?.and(0xff) in 92..95 -> configurationResponse(request)
            else -> null
        } ?: return null
        return encodeFrame(payload)
    }

    private fun configurationResponse(request: ByteArray): ByteArray? {
        return when (request.first().toInt() and 0xff) {
        GET_CONFIGURATION_XML -> {
            if (request.size != 10 || request[1].toInt() != 0) return null
            val length = i32(request, 2) ?: return null
            val offset = i32(request, 6) ?: return null
            if (length !in 1..400 || offset !in configurationSchema.indices) return null
            byteArrayOf(GET_CONFIGURATION_XML.toByte(), 0) +
                i32Bytes(configurationSchema.size) + i32Bytes(offset) +
                configurationSchema.copyOfRange(offset, minOf(configurationSchema.size, offset + length))
        }
        GET_CONFIGURATION -> {
            if (!request.contentEquals(byteArrayOf(GET_CONFIGURATION.toByte(), 0))) return null
            byteArrayOf(GET_CONFIGURATION.toByte(), 0) + currentConfiguration
        }
        GET_CONFIGURATION_DEFAULT -> {
            if (!request.contentEquals(byteArrayOf(GET_CONFIGURATION_DEFAULT.toByte(), 0))) return null
            byteArrayOf(GET_CONFIGURATION_DEFAULT.toByte(), 0) + defaultConfiguration
        }
        SET_CONFIGURATION -> {
            if (request.size <= 2 || request[1].toInt() != 0 ||
                !request.copyOfRange(2, request.size).contentEquals(acceptedConfigurationWrite)
            ) return null
            currentConfiguration = expectedConfigurationReadback.copyOf()
            byteArrayOf(SET_CONFIGURATION.toByte())
        }
        else -> null
        }
    }

    private companion object {
        const val GET_FIRMWARE = 0
        const val FORWARD_CAN = 34
        const val CUSTOM_APP_DATA = 36
        const val PING_CAN = 62
        const val GET_CONFIGURATION_XML = 92
        const val GET_CONFIGURATION = 93
        const val GET_CONFIGURATION_DEFAULT = 94
        const val SET_CONFIGURATION = 95
        const val BMS_VALUES = 96
        const val PACKAGE_ID = 101
        const val INFO: Byte = 0
        const val GET_ALL_DATA: Byte = 10
        const val BMS_CAN_ID = 42
        val firmware = byteArrayOf(GET_FIRMWARE.toByte(), 6, 6) + "THOR300".encodeToByteArray() +
            byteArrayOf(0) + ByteArray(12) + byteArrayOf(0, 0, 0, 1, 0, 0, 0, 0) +
            "VESC".encodeToByteArray() + byteArrayOf(0)
        val telemetry = ByteArray(55).apply {
            this[0] = CUSTOM_APP_DATA.toByte(); this[1] = PACKAGE_ID.toByte(); this[2] = GET_ALL_DATA
            this[3] = 3; this[10] = 1; this[23] = 2; this[24] = 4
            this[33] = 128.toByte(); this[39] = 60; this[40] = 56
        }
        val bmsFirmware = "00060056424d530000112233445566778899aabb00000101".hexBytes()
        val bmsValues =
            "6003010b0003412140ff43a158ff445120000004e20000f424031068105e10540100010209f60a410abe09ab14820b6d036b03b62a428000004480000042000000440000002794014f4b00".hexBytes()

        fun String.hexBytes() = chunked(2).map { it.toInt(16).toByte() }.toByteArray()

        fun encodeFrame(payload: ByteArray): ByteArray {
            val header = if (payload.size <= 255) byteArrayOf(2, payload.size.toByte())
                else byteArrayOf(3, (payload.size ushr 8).toByte(), payload.size.toByte())
            val checksum = crc16(payload)
            return header + payload + byteArrayOf((checksum.toInt() ushr 8).toByte(), checksum.toByte(), 3)
        }

        fun decodeFrame(frame: ByteArray): ByteArray? {
            if (frame.isEmpty()) return null
            val headerCount: Int
            val payloadCount: Int
            when (frame[0].toInt()) {
                2 -> { if (frame.size < 5) return null; headerCount = 2; payloadCount = frame[1].toInt() and 0xff }
                3 -> { if (frame.size < 6) return null; headerCount = 3
                    payloadCount = (frame[1].toInt() and 0xff) shl 8 or (frame[2].toInt() and 0xff) }
                else -> return null
            }
            if (payloadCount > 4_096 || frame.size != headerCount + payloadCount + 3 || frame.last().toInt() != 3) return null
            val payload = frame.copyOfRange(headerCount, headerCount + payloadCount)
            val received = (frame[headerCount + payloadCount].toInt() and 0xff) shl 8 or
                (frame[headerCount + payloadCount + 1].toInt() and 0xff)
            return payload.takeIf { crc16(it).toInt() == received }
        }

        fun crc16(bytes: ByteArray): UShort {
            var crc = 0
            bytes.forEach { byte ->
                crc = crc xor ((byte.toInt() and 0xff) shl 8)
                repeat(8) { crc = if (crc and 0x8000 == 0) crc shl 1 else (crc shl 1) xor 0x1021 }
                crc = crc and 0xffff
            }
            return crc.toUShort()
        }
    }
}

private fun loadDevelopmentFixture(path: String): ByteArray {
    val root = System.getProperty("floatViewFixtureRoot")
        ?: throw IllegalStateException("Software Refloat configuration fixtures are unavailable")
    return File(root, "refloat-configuration/$path").readBytes()
}

private fun ByteArray.bounded(maximum: Int) = also {
    require(isNotEmpty() && size <= maximum)
}

private fun i32(bytes: ByteArray, offset: Int): Int? {
    if (offset + 4 > bytes.size) return null
    val value = ((bytes[offset].toLong() and 0xff) shl 24) or
        ((bytes[offset + 1].toLong() and 0xff) shl 16) or
        ((bytes[offset + 2].toLong() and 0xff) shl 8) or (bytes[offset + 3].toLong() and 0xff)
    return value.takeIf { it <= Int.MAX_VALUE }?.toInt()
}

private fun i32Bytes(value: Int) = byteArrayOf(
    (value ushr 24).toByte(), (value ushr 16).toByte(), (value ushr 8).toByte(), value.toByte(),
)
