package fail.acheron.floatview.protocol.vesc

import fail.acheron.floatview.protocol.FirmwareInfo
import fail.acheron.floatview.protocol.VescProtocolException

/** Strict owner for the source-defined COMM_FW_VERSION identity and optional CRC tail. */
object VescFirmwareIdentityDecoder {
    fun decode(payload: ByteArray): FirmwareInfo {
        val command = payload.firstOrNull()?.toInt()?.and(0xff)
            ?: throw VescProtocolException.TruncatedPayload
        if (command != 0) throw VescProtocolException.UnsupportedCommand(command)
        if (payload.size < 3) throw VescProtocolException.TruncatedPayload

        fun terminatedBytes(start: Int, maximum: Int): ByteArray {
            if (start >= payload.size) throw VescProtocolException.TruncatedPayload
            val end = (start until payload.size).firstOrNull { payload[it].toInt() == 0 }
                ?: throw VescProtocolException.TruncatedPayload
            if (end - start > maximum) throw VescProtocolException.TruncatedPayload
            return payload.copyOfRange(start, end)
        }

        val hardwareNameBytes = terminatedBytes(3, 64)
        val uuidStart = hardwareNameBytes.size + 4
        val flagsStart = uuidStart + 12
        val firmwareNameStart = flagsStart + 8
        if (firmwareNameStart >= payload.size) throw VescProtocolException.TruncatedPayload
        val controllerUuid = payload.copyOfRange(uuidStart, flagsStart)
        val firmwareNameBytes = terminatedBytes(firmwareNameStart, 64)
        val tailLength = payload.size - (firmwareNameStart + firmwareNameBytes.size + 1)
        if (tailLength != 0 && tailLength != 4) throw VescProtocolException.TruncatedPayload
        return FirmwareInfo(
            payload[1].toInt() and 0xff,
            payload[2].toInt() and 0xff,
            hardwareNameBytes.decodeToString(),
            firmwareNameBytes.decodeToString().ifEmpty { null },
            payload[uuidStart + 14].toInt() and 0xff,
            controllerUuid,
            payload[uuidStart + 13].toInt() and 0xff,
        )
    }
}
