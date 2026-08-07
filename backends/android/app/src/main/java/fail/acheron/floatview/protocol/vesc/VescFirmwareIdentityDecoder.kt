package fail.acheron.floatview.protocol.vesc

import fail.acheron.floatview.protocol.FirmwareInfo
import fail.acheron.floatview.protocol.VescProtocolException

/** Strict owner for the bounded common COMM_FW_VERSION prefix and hardware-specific tails. */
object VescFirmwareIdentityDecoder {
    const val MOTOR_HARDWARE_TYPE = 0
    const val BMS_HARDWARE_TYPE = 1

    fun decode(payload: ByteArray): FirmwareInfo {
        val command = payload.firstOrNull()?.toInt()?.and(0xff)
            ?: throw VescProtocolException.TruncatedPayload
        if (command != VescCommand.GET_FIRMWARE_VERSION.id) {
            throw VescProtocolException.UnsupportedCommand(command)
        }
        if (payload.size < 3) throw VescProtocolException.TruncatedPayload

        val hardwareNameBytes = terminatedBytes(payload, 3, 64)
        val uuidStart = hardwareNameBytes.size + 4
        val commonEnd = uuidStart + 16
        if (commonEnd > payload.size) throw VescProtocolException.TruncatedPayload
        val hardwareType = payload[uuidStart + 14].toInt() and 0xff
        val customConfigurationCount = payload[uuidStart + 15].toInt() and 0xff
        val extended = when (hardwareType) {
            BMS_HARDWARE_TYPE -> {
                if (
                    (payload[1].toInt() and 0xff) != 6 ||
                    (payload[2].toInt() and 0xff) != 0 ||
                    payload.size != commonEnd
                ) throw VescProtocolException.TruncatedPayload
                null to null
            }
            MOTOR_HARDWARE_TYPE -> extendedFirmwareName(payload, commonEnd)
            else -> extendedFirmwareName(payload, commonEnd)
        }
        return FirmwareInfo(
            payload[1].toInt() and 0xff,
            payload[2].toInt() and 0xff,
            hardwareNameBytes.decodeToString(),
            extended.first,
            hardwareType,
            payload.copyOfRange(uuidStart, uuidStart + 12),
            payload[uuidStart + 13].toInt() and 0xff,
            customConfigurationCount,
            extended.second,
        )
    }

    private fun extendedFirmwareName(payload: ByteArray, commonEnd: Int): Pair<String?, Long?> {
        val start = commonEnd + 4
        if (start >= payload.size) throw VescProtocolException.TruncatedPayload
        val bytes = terminatedBytes(payload, start, 64)
        val tailLength = payload.size - (start + bytes.size + 1)
        if (tailLength != 0 && tailLength != 4) throw VescProtocolException.TruncatedPayload
        val crc = if (tailLength == 4) {
            var value = 0L
            for (index in payload.size - 4 until payload.size) value = (value shl 8) or (payload[index].toLong() and 0xff)
            value
        } else null
        return bytes.decodeToString().ifEmpty { null } to crc
    }

    private fun terminatedBytes(payload: ByteArray, start: Int, maximum: Int): ByteArray {
        if (start >= payload.size) throw VescProtocolException.TruncatedPayload
        val end = (start until payload.size).firstOrNull { payload[it].toInt() == 0 }
            ?: throw VescProtocolException.TruncatedPayload
        if (end - start > maximum) throw VescProtocolException.TruncatedPayload
        return payload.copyOfRange(start, end)
    }
}
