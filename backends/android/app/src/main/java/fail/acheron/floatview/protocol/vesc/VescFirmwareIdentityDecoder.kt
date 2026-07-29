package fail.acheron.floatview.protocol.vesc

import fail.acheron.floatview.protocol.FirmwareInfo
import fail.acheron.floatview.protocol.VescProtocolException

/** Length-checked owner for COMM_FW_VERSION identity and optional tail fields. */
object VescFirmwareIdentityDecoder {
    fun decode(payload: ByteArray): FirmwareInfo {
        if (payload.size < 4 || payload[0].toInt() != 0) {
            throw VescProtocolException.UnsupportedCommand(payload.firstOrNull()?.toInt() ?: 255)
        }
        val end = (3 until payload.size).firstOrNull { payload[it].toInt() == 0 } ?: payload.size
        var offset = minOf(end + 1, payload.size)
        val controllerUuid =
            if (payload.size - offset >= 12) {
                payload.copyOfRange(offset, offset + 12).also { offset += 12 }
            } else {
                null
            }
        val hardwareType =
            if (payload.size - offset >= 3) payload[offset + 2].toInt() and 0xff else 0
        repeat(8) {
            if (offset < payload.size) offset++
        }
        val firmwareName =
            if (offset < payload.size) {
                val firmwareNameEnd =
                    (offset until payload.size).firstOrNull { payload[it].toInt() == 0 }
                        ?: payload.size
                payload.copyOfRange(offset, firmwareNameEnd).decodeToString().ifEmpty { null }
            } else {
                null
            }
        return FirmwareInfo(
            payload[1].toInt() and 0xff,
            payload[2].toInt() and 0xff,
            payload.copyOfRange(3, end).decodeToString(),
            firmwareName,
            hardwareType,
            controllerUuid,
        )
    }
}
