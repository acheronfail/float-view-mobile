package fail.acheron.floatview.protocol

sealed class VescProtocolException(message: String) : Exception(message) {
    data object InvalidFrame : VescProtocolException("Invalid VESC frame")
    data object InvalidCrc : VescProtocolException("Invalid VESC CRC")
    data object PayloadTooLarge : VescProtocolException("VESC payload exceeds limit")
    class UnsupportedCommand(val command: Int) : VescProtocolException("Unsupported command $command")
    data object TruncatedPayload : VescProtocolException("Truncated VESC payload")
}

data class FirmwareInfo(val major: Int, val minor: Int, val hardwareName: String)

data class RealtimeValues(
    val controllerTempC: Double,
    val motorTempC: Double,
    val motorCurrentA: Double,
    val batteryCurrentA: Double,
    val dutyCycle: Double,
    val erpm: Int,
    val packVoltageV: Double,
    val faultCode: Int,
)

data class RefloatRealtime(
    val appId: Int,
    val packetId: Int,
    val rideState: Int,
    val footpadState: Int,
)

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

private class ByteReader(private val bytes: ByteArray) {
    private var offset = 0
    fun u8(): Int {
        if (offset >= bytes.size) throw VescProtocolException.TruncatedPayload
        return bytes[offset++].toInt() and 0xff
    }
    fun i16(scale: Double): Double {
        val value = ((u8() shl 8) or u8()).toShort()
        return value / scale
    }
    fun i32(scale: Double = 1.0): Double {
        var value = 0
        repeat(4) { value = (value shl 8) or u8() }
        return value / scale
    }
    fun skip(count: Int) {
        if (offset + count > bytes.size) throw VescProtocolException.TruncatedPayload
        offset += count
    }
}

object VescPayloadDecoder {
    fun firmware(payload: ByteArray): FirmwareInfo {
        if (payload.size < 4 || payload[0].toInt() != 0) {
            throw VescProtocolException.UnsupportedCommand(payload.firstOrNull()?.toInt() ?: 255)
        }
        val end = (3 until payload.size).firstOrNull { payload[it].toInt() == 0 } ?: payload.size
        return FirmwareInfo(
            payload[1].toInt() and 0xff,
            payload[2].toInt() and 0xff,
            payload.copyOfRange(3, end).decodeToString(),
        )
    }

    fun realtime(payload: ByteArray): RealtimeValues {
        val reader = ByteReader(payload)
        val command = reader.u8()
        if (command != 4) throw VescProtocolException.UnsupportedCommand(command)
        val controllerTemp = reader.i16(10.0)
        val motorTemp = reader.i16(10.0)
        val motorCurrent = reader.i32(100.0)
        val batteryCurrent = reader.i32(100.0)
        reader.skip(8)
        val duty = reader.i16(1000.0)
        val erpm = reader.i32().toInt()
        val voltage = reader.i16(10.0)
        val fault = reader.u8()
        return RealtimeValues(
            controllerTemp, motorTemp, motorCurrent, batteryCurrent, duty, erpm, voltage, fault,
        )
    }

    fun refloatRealtime(payload: ByteArray): RefloatRealtime {
        if (payload.size < 5 || payload[0].toInt() != 36) {
            throw VescProtocolException.UnsupportedCommand(payload.firstOrNull()?.toInt() ?: 255)
        }
        return RefloatRealtime(
            payload[1].toInt() and 0xff,
            payload[2].toInt() and 0xff,
            payload[3].toInt() and 0xff,
            payload[4].toInt() and 0xff,
        )
    }
}
