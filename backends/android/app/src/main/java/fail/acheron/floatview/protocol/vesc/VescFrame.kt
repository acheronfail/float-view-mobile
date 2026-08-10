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
    private const val ABI_VERSION = 2
    private const val STATUS_OK = 0
    private const val STATUS_INVALID_ARGUMENT = 1
    private const val STATUS_INPUT_TOO_LARGE = 2
    private const val STATUS_INVALID_FRAME = 3
    private const val STATUS_INVALID_CRC = 4
    private const val STATUS_OUTPUT_TOO_SMALL = 5
    private const val STATUS_INTERNAL_FAILURE = 6
    private const val STATUS_NEED_MORE_INPUT = 7

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

    internal fun scanStream(bytes: ByteArray): VescFrameStreamScan {
        if (linkedAbiVersion != ABI_VERSION) throw VescProtocolException.InvalidFrame
        val scanResult = IntArray(3)
        val status = RustVescFrameBridge.scanNext(bytes, scanResult)
        return decodeStreamScan(status, scanResult, bytes.size)
    }

    internal fun decodeStreamScan(
        status: Int,
        scanResult: IntArray,
        inputSize: Int,
    ): VescFrameStreamScan {
        if (scanResult.size != 3 || inputSize < 0) throw VescProtocolException.InvalidFrame
        val (frameOffset, frameLength, consumed) = scanResult
        return when (status) {
            STATUS_OK -> {
                val frameEnd = frameOffset.toLong() + frameLength.toLong()
                if (
                    frameOffset < 0 ||
                    frameLength <= 0 ||
                    frameEnd != consumed.toLong() ||
                    frameEnd > inputSize.toLong()
                ) {
                    throw VescProtocolException.InvalidFrame
                }
                VescFrameStreamScan.Frame(frameOffset, frameLength, consumed)
            }
            STATUS_NEED_MORE_INPUT -> {
                if (frameOffset != 0 || frameLength != 0 || consumed !in 0..inputSize) {
                    throw VescProtocolException.InvalidFrame
                }
                VescFrameStreamScan.NeedMore(consumed)
            }
            STATUS_INPUT_TOO_LARGE -> {
                if (frameOffset != 0 || frameLength != 0 || consumed !in 1..inputSize) {
                    throw VescProtocolException.InvalidFrame
                }
                VescFrameStreamScan.Failure(VescProtocolException.PayloadTooLarge, consumed)
            }
            STATUS_INVALID_ARGUMENT,
            STATUS_INVALID_FRAME,
            STATUS_INVALID_CRC,
            STATUS_OUTPUT_TOO_SMALL,
            STATUS_INTERNAL_FAILURE,
            -> throw VescProtocolException.InvalidFrame
            else -> throw VescProtocolException.InvalidFrame
        }
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

internal sealed interface VescFrameStreamScan {
    data class NeedMore(val consumed: Int) : VescFrameStreamScan

    data class Frame(
        val offset: Int,
        val length: Int,
        val consumed: Int,
    ) : VescFrameStreamScan

    data class Failure(
        val error: VescProtocolException,
        val consumed: Int,
    ) : VescFrameStreamScan
}

class VescFrameStreamDecoder {
    private val buffer = ArrayList<Byte>()

    fun append(bytes: ByteArray): List<ByteArray> {
        buffer.addAll(bytes.toList())
        val results = mutableListOf<ByteArray>()
        while (true) {
            when (val scan = VescFrameCodec.scanStream(buffer.toByteArray())) {
                is VescFrameStreamScan.NeedMore -> {
                    removePrefix(scan.consumed)
                    return results
                }
                is VescFrameStreamScan.Frame -> {
                    val frame = buffer.subList(scan.offset, scan.offset + scan.length).toByteArray()
                    removePrefix(scan.consumed)
                    results += VescFrameCodec.decode(frame)
                }
                is VescFrameStreamScan.Failure -> {
                    removePrefix(scan.consumed)
                    throw scan.error
                }
            }
        }
    }

    private fun removePrefix(consumed: Int) {
        if (consumed == buffer.size) buffer.clear() else repeat(consumed) { buffer.removeAt(0) }
    }
}
