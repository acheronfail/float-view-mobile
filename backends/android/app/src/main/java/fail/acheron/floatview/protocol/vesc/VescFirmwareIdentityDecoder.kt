package fail.acheron.floatview.protocol.vesc

import fail.acheron.floatview.protocol.FirmwareInfo
import fail.acheron.floatview.protocol.RustVescFirmwareIdentityBridge
import fail.acheron.floatview.protocol.VescProtocolException
import fail.acheron.floatview.protocol.verifyVescFirmwareIdentityAbiVersion
import java.nio.ByteBuffer
import java.nio.charset.CharacterCodingException
import java.nio.charset.CodingErrorAction
import java.nio.charset.StandardCharsets

/** Android adapter for the Rust-owned COMM_FW_VERSION structural decoder. */
object VescFirmwareIdentityDecoder {
    const val MOTOR_HARDWARE_TYPE = 0
    const val BMS_HARDWARE_TYPE = 1

    private const val RECORD_HEADER_BYTES = 26
    private const val MAXIMUM_ENCODED_NAME_BYTES = 192
    private const val MAXIMUM_RECORD_BYTES = 410
    private const val FIRMWARE_NAME_PRESENT = 1
    private const val HARDWARE_CRC_PRESENT = 2
    private const val KNOWN_FLAGS = FIRMWARE_NAME_PRESENT or HARDWARE_CRC_PRESENT
    private const val STATUS_OK = 0
    private const val STATUS_UNSUPPORTED_COMMAND = 2

    private val linkedAbiVersion = RustVescFirmwareIdentityBridge.abiVersion()

    fun decode(payload: ByteArray): FirmwareInfo {
        verifyVescFirmwareIdentityAbiVersion(linkedAbiVersion)
        val record = ByteArray(MAXIMUM_RECORD_BYTES)
        val result = IntArray(2)
        val status = RustVescFirmwareIdentityBridge.decode(payload, record, result)
        return decodeBridgeResult(status, record, result)
    }

    internal fun decodeBridgeResult(
        status: Int,
        record: ByteArray,
        result: IntArray,
    ): FirmwareInfo {
        if (result.size != 2) throw VescProtocolException.TruncatedPayload
        val (recordLength, unsupportedCommand) = result
        return when (status) {
            STATUS_OK -> {
                if (
                    recordLength !in RECORD_HEADER_BYTES..minOf(record.size, MAXIMUM_RECORD_BYTES) ||
                    unsupportedCommand != 0
                ) throw VescProtocolException.TruncatedPayload
                firmwareInfo(record.copyOf(recordLength))
            }
            STATUS_UNSUPPORTED_COMMAND -> {
                if (recordLength != 0 || unsupportedCommand !in 1..255) {
                    throw VescProtocolException.TruncatedPayload
                }
                throw VescProtocolException.UnsupportedCommand(unsupportedCommand)
            }
            else -> throw VescProtocolException.TruncatedPayload
        }
    }

    internal fun firmwareInfo(record: ByteArray): FirmwareInfo {
        if (record.size !in RECORD_HEADER_BYTES..MAXIMUM_RECORD_BYTES) {
            throw VescProtocolException.TruncatedPayload
        }
        val flags = record[5].unsigned()
        if (flags and KNOWN_FLAGS.inv() != 0) throw VescProtocolException.TruncatedPayload
        val hardwareNameLength = record.bigEndianUnsignedShort(6)
        val firmwareNameLength = record.bigEndianUnsignedShort(8)
        if (
            hardwareNameLength > MAXIMUM_ENCODED_NAME_BYTES ||
            firmwareNameLength > MAXIMUM_ENCODED_NAME_BYTES ||
            RECORD_HEADER_BYTES + hardwareNameLength + firmwareNameLength != record.size ||
            (flags and FIRMWARE_NAME_PRESENT != 0) != (firmwareNameLength > 0)
        ) throw VescProtocolException.TruncatedPayload

        val hasCrc = flags and HARDWARE_CRC_PRESENT != 0
        val crcBytes = record.copyOfRange(22, 26)
        if (!hasCrc && crcBytes.any { it.toInt() != 0 }) {
            throw VescProtocolException.TruncatedPayload
        }
        val hardwareNameEnd = RECORD_HEADER_BYTES + hardwareNameLength
        val hardwareName = record.strictUtf8(RECORD_HEADER_BYTES, hardwareNameEnd)
        val firmwareName = if (firmwareNameLength == 0) {
            null
        } else {
            record.strictUtf8(hardwareNameEnd, record.size)
        }
        val crc = if (hasCrc) {
            crcBytes.fold(0L) { value, byte -> (value shl 8) or byte.unsigned().toLong() }
        } else {
            null
        }
        return FirmwareInfo(
            major = record[0].unsigned(),
            minor = record[1].unsigned(),
            hardwareName = hardwareName,
            firmwareName = firmwareName,
            hardwareType = record[3].unsigned(),
            controllerUuid = record.copyOfRange(10, 22),
            testVersion = record[2].unsigned(),
            customConfigurationCount = record[4].unsigned(),
            hardwareConfigurationCrc = crc,
        )
    }

    private fun Byte.unsigned(): Int = toInt() and 0xff

    private fun ByteArray.bigEndianUnsignedShort(offset: Int): Int =
        (this[offset].unsigned() shl 8) or this[offset + 1].unsigned()

    private fun ByteArray.strictUtf8(start: Int, end: Int): String {
        try {
            return StandardCharsets.UTF_8.newDecoder()
                .onMalformedInput(CodingErrorAction.REPORT)
                .onUnmappableCharacter(CodingErrorAction.REPORT)
                .decode(ByteBuffer.wrap(this, start, end - start))
                .toString()
        } catch (_: CharacterCodingException) {
            throw VescProtocolException.TruncatedPayload
        }
    }
}
