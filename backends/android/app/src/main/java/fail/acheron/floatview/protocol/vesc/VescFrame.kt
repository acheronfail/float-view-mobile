package fail.acheron.floatview.protocol

sealed class VescProtocolException(message: String) : Exception(message) {
    data object InvalidFrame : VescProtocolException("Invalid VESC frame")
    data object InvalidCrc : VescProtocolException("Invalid VESC CRC")
    data object PayloadTooLarge : VescProtocolException("VESC payload exceeds limit")
    class UnsupportedCommand(val command: Int) : VescProtocolException("Unsupported command $command")
    data object TruncatedPayload : VescProtocolException("Truncated VESC payload")
}

object VescFrameCodec {
    const val MAXIMUM_PAYLOAD_BYTES = 4096
    private const val MAXIMUM_FRAME_BYTES = MAXIMUM_PAYLOAD_BYTES + 6
    private const val ABI_VERSION = 1
    private const val STATUS_OK = 0
    private const val STATUS_INVALID_ARGUMENT = 1
    private const val STATUS_INPUT_TOO_LARGE = 2
    private const val STATUS_INVALID_FRAME = 3
    private const val STATUS_INVALID_CRC = 4
    private const val STATUS_OUTPUT_TOO_SMALL = 5
    private const val STATUS_INTERNAL_FAILURE = 6

    private val linkedAbiVersion = RustVescFrameBridge.abiVersion()

    fun encode(payload: ByteArray): ByteArray {
        if (linkedAbiVersion != ABI_VERSION) throw VescProtocolException.InvalidFrame
        val output = ByteArray(MAXIMUM_FRAME_BYTES)
        val outputLength = IntArray(1)
        return result(RustVescFrameBridge.encode(payload, output, outputLength), output, outputLength)
    }

    fun decode(frame: ByteArray): ByteArray {
        if (linkedAbiVersion != ABI_VERSION) throw VescProtocolException.InvalidFrame
        val output = ByteArray(MAXIMUM_PAYLOAD_BYTES)
        val outputLength = IntArray(1)
        return result(RustVescFrameBridge.decodeExact(frame, output, outputLength), output, outputLength)
    }

    private fun result(status: Int, output: ByteArray, outputLength: IntArray): ByteArray {
        val produced = outputLength.single()
        return when (status) {
            STATUS_OK -> {
                if (produced !in 0..output.size) throw VescProtocolException.InvalidFrame
                output.copyOf(produced)
            }
            STATUS_INPUT_TOO_LARGE -> throw VescProtocolException.PayloadTooLarge
            STATUS_INVALID_FRAME -> throw VescProtocolException.InvalidFrame
            STATUS_INVALID_CRC -> throw VescProtocolException.InvalidCrc
            STATUS_INVALID_ARGUMENT,
            STATUS_OUTPUT_TOO_SMALL,
            STATUS_INTERNAL_FAILURE,
            -> throw VescProtocolException.InvalidFrame
            else -> throw VescProtocolException.InvalidFrame
        }
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
