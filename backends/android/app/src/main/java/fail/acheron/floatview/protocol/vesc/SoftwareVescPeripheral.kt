package fail.acheron.floatview.protocol.vesc

import fail.acheron.floatview.protocol.VescFrameCodec

/** Minimal deterministic direct VESC used only by software-host conformance. */
class SoftwareVescPeripheral {
    fun response(frame: ByteArray): ByteArray? {
        val request = runCatching { VescFrameCodec.decode(frame) }.getOrNull() ?: return null
        val payload =
            when {
                request.contentEquals(byteArrayOf(0)) -> firmware
                request.contentEquals(byteArrayOf(36, 101, 0, 2, 0)) ->
                    byteArrayOf(36, 101, 0, 10)
                request.contentEquals(byteArrayOf(36, 101, 10)) -> telemetry
                else -> null
            } ?: return null
        return VescFrameCodec.encode(payload)
    }

    private companion object {
        val firmware =
            byteArrayOf(0, 6, 6) +
                "THOR300".encodeToByteArray() +
                byteArrayOf(0) +
                ByteArray(20)
        val telemetry =
            ByteArray(55).apply {
                this[0] = 36
                this[1] = 101
                this[2] = 10
                this[3] = 3
                this[23] = 2
                this[24] = 4
                this[33] = 128.toByte()
                this[39] = 60
                this[40] = 56
            }
    }
}
