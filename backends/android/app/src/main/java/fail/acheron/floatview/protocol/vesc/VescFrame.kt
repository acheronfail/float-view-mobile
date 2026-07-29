package fail.acheron.floatview.protocol

sealed class VescProtocolException(message: String) : Exception(message) {
    data object InvalidFrame : VescProtocolException("Invalid VESC frame")
    data object InvalidCrc : VescProtocolException("Invalid VESC CRC")
    data object PayloadTooLarge : VescProtocolException("VESC payload exceeds limit")
    class UnsupportedCommand(val command: Int) : VescProtocolException("Unsupported command $command")
    data object TruncatedPayload : VescProtocolException("Truncated VESC payload")
}

object VescCrc {
    fun checksum(bytes: ByteArray): Int {
        var crc = 0
        for (byte in bytes) {
            crc = crc xor ((byte.toInt() and 0xff) shl 8)
            repeat(8) {
                crc = if ((crc and 0x8000) != 0) ((crc shl 1) xor 0x1021) else crc shl 1
                crc = crc and 0xffff
            }
        }
        return crc
    }
}

object VescFrameCodec {
    const val MAXIMUM_PAYLOAD_BYTES = 4096

    fun encode(payload: ByteArray): ByteArray {
        if (payload.size > MAXIMUM_PAYLOAD_BYTES) throw VescProtocolException.PayloadTooLarge
        val header =
            if (payload.size <= 255) byteArrayOf(2, payload.size.toByte())
            else byteArrayOf(3, (payload.size shr 8).toByte(), payload.size.toByte())
        val crc = VescCrc.checksum(payload)
        return header + payload + byteArrayOf((crc shr 8).toByte(), crc.toByte(), 3)
    }

    fun decode(frame: ByteArray): ByteArray {
        if (frame.isEmpty()) throw VescProtocolException.InvalidFrame
        val headerSize: Int
        val payloadSize: Int
        when (frame[0].toInt() and 0xff) {
            2 -> {
                if (frame.size < 5) throw VescProtocolException.InvalidFrame
                headerSize = 2
                payloadSize = frame[1].toInt() and 0xff
            }
            3 -> {
                if (frame.size < 6) throw VescProtocolException.InvalidFrame
                headerSize = 3
                payloadSize = ((frame[1].toInt() and 0xff) shl 8) or
                    (frame[2].toInt() and 0xff)
            }
            else -> throw VescProtocolException.InvalidFrame
        }
        if (payloadSize > MAXIMUM_PAYLOAD_BYTES) throw VescProtocolException.PayloadTooLarge
        if (frame.size != headerSize + payloadSize + 3 || frame.last().toInt() != 3) {
            throw VescProtocolException.InvalidFrame
        }
        val payload = frame.copyOfRange(headerSize, headerSize + payloadSize)
        val received = ((frame[headerSize + payloadSize].toInt() and 0xff) shl 8) or
            (frame[headerSize + payloadSize + 1].toInt() and 0xff)
        if (VescCrc.checksum(payload) != received) throw VescProtocolException.InvalidCrc
        return payload
    }
}

class VescFrameStreamDecoder {
    private val buffer = ArrayList<Byte>()

    fun append(bytes: ByteArray): List<ByteArray> {
        buffer.addAll(bytes.toList())
        if (buffer.size > VescFrameCodec.MAXIMUM_PAYLOAD_BYTES * 2) {
            buffer.clear()
            throw VescProtocolException.PayloadTooLarge
        }
        val results = mutableListOf<ByteArray>()
        while (true) {
            while (buffer.isNotEmpty() && buffer.first().toInt() !in setOf(2, 3)) {
                buffer.removeAt(0)
            }
            if (buffer.isEmpty()) return results
            val headerSize = if (buffer.first().toInt() == 2) 2 else 3
            if (buffer.size < headerSize) return results
            val size =
                if (headerSize == 2) buffer[1].toInt() and 0xff
                else ((buffer[1].toInt() and 0xff) shl 8) or (buffer[2].toInt() and 0xff)
            if (size > VescFrameCodec.MAXIMUM_PAYLOAD_BYTES) {
                buffer.removeAt(0)
                throw VescProtocolException.PayloadTooLarge
            }
            val frameSize = headerSize + size + 3
            if (buffer.size < frameSize) return results
            val frame = buffer.subList(0, frameSize).toByteArray()
            repeat(frameSize) { buffer.removeAt(0) }
            results += VescFrameCodec.decode(frame)
        }
    }
}
