package fail.acheron.floatview.protocol

import fail.acheron.floatview.backend.FirmwareRecord
import fail.acheron.floatview.backend.VescProtocolMessage
import fail.acheron.floatview.backend.VescProtocolPort
import fail.acheron.floatview.backend.RefloatTelemetryRecord
import fail.acheron.floatview.backend.RideState

sealed class VescProtocolException(message: String) : Exception(message) {
    data object InvalidFrame : VescProtocolException("Invalid VESC frame")
    data object InvalidCrc : VescProtocolException("Invalid VESC CRC")
    data object PayloadTooLarge : VescProtocolException("VESC payload exceeds limit")
    class UnsupportedCommand(val command: Int) : VescProtocolException("Unsupported command $command")
    data object TruncatedPayload : VescProtocolException("Truncated VESC payload")
}

data class FirmwareInfo(
    val major: Int,
    val minor: Int,
    val hardwareName: String,
    val firmwareName: String? = null,
    val hardwareType: Int = 0,
)

data class RealtimeValues(
    val controllerTempC: Double,
    val motorTempC: Double,
    val motorCurrentA: Double,
    val batteryCurrentA: Double,
    val dutyCycle: Double,
    val erpm: Int,
    val packVoltageV: Double,
    val faultCode: Int,
    val fault: VescFault = VescFault.fromCode(faultCode),
)

data class VescFault(val code: Int, val kind: Kind) {
    enum class Kind(val stableName: String) {
        NONE("none"),
        OVER_VOLTAGE("over_voltage"),
        UNKNOWN("unknown"),
    }

    companion object {
        fun fromCode(code: Int): VescFault =
            VescFault(
                code,
                when (code) {
                    0 -> Kind.NONE
                    1 -> Kind.OVER_VOLTAGE
                    else -> Kind.UNKNOWN
                },
            )
    }
}

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
        var offset = minOf(end + 1, payload.size)
        if (payload.size - offset >= 12) offset += 12 // UUID
        val hardwareType =
            if (payload.size - offset >= 3) payload[offset + 2].toInt() and 0xff else 0
        repeat(8) {
            if (offset < payload.size) offset++ // paired through NRF flags
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

class VescProtocolService : VescProtocolPort {
    private var stream = VescFrameStreamDecoder()
    private var pendingFirmwareCanId: Int? = null
    private var realtimeFields = emptyList<String>()
    private var realtimeRuntimeFields = emptyList<String>()

    @Synchronized
    override fun reset() {
        stream = VescFrameStreamDecoder()
        pendingFirmwareCanId = null
        realtimeFields = emptyList()
        realtimeRuntimeFields = emptyList()
    }

    @Synchronized
    override fun firmwareRequest(canId: Int?): ByteArray {
        require(canId == null || canId in 0..255) { "CAN ID must fit in one byte" }
        pendingFirmwareCanId = canId
        val payload =
            if (canId == null) byteArrayOf(0)
            else byteArrayOf(34, canId.toByte(), 0)
        return VescFrameCodec.encode(payload)
    }

    override fun canPingRequest(): ByteArray = VescFrameCodec.encode(byteArrayOf(62))

    override fun refloatInfoRequest(canId: Int?): ByteArray {
        require(canId == null || canId in 0..255) { "CAN ID must fit in one byte" }
        val command = byteArrayOf(36, 101, 0, 2, 0)
        val payload =
            if (canId == null) command
            else byteArrayOf(34, canId.toByte()) + command
        return VescFrameCodec.encode(payload)
    }

    override fun refloatTelemetryIdsRequest(canId: Int): ByteArray {
        require(canId in 0..255) { "CAN ID must fit in one byte" }
        return VescFrameCodec.encode(byteArrayOf(34, canId.toByte(), 36, 101, 32))
    }

    override fun refloatTelemetryRequest(canId: Int): ByteArray {
        require(canId in 0..255) { "CAN ID must fit in one byte" }
        return VescFrameCodec.encode(byteArrayOf(34, canId.toByte(), 36, 101, 31))
    }

    @Synchronized
    override fun receive(bytes: ByteArray): List<VescProtocolMessage> =
        stream.append(bytes).mapNotNull { payload ->
            when (payload.firstOrNull()?.toInt()?.and(0xff)) {
                0 -> {
                    val info = VescPayloadDecoder.firmware(payload)
                    VescProtocolMessage.Firmware(
                        FirmwareRecord(
                            info.major,
                            info.minor,
                            info.hardwareName,
                            info.firmwareName,
                            info.hardwareType,
                            pendingFirmwareCanId,
                        ),
                    )
                }
                62 -> VescProtocolMessage.CanDevices(
                    payload.drop(1).map { it.toInt() and 0xff }.distinct().sorted(),
                )
                36 -> {
                    if (
                        payload.size >= 4 &&
                        payload[1].toInt() and 0xff == 101 &&
                        payload[2].toInt() and 0xff == 0
                    ) {
                        VescProtocolMessage.RefloatInfo(decodeRefloatVersion(payload))
                    } else if (
                        payload.size >= 5 &&
                        payload[1].toInt() and 0xff == 101 &&
                        payload[2].toInt() and 0xff == 32
                    ) {
                        val (fields, runtimeFields) = decodeRealtimeFields(payload)
                        realtimeFields = fields
                        realtimeRuntimeFields = runtimeFields
                        VescProtocolMessage.RefloatTelemetryLayout(fields, runtimeFields)
                    } else if (
                        payload.size >= 13 &&
                        payload[1].toInt() and 0xff == 101 &&
                        payload[2].toInt() and 0xff == 31
                    ) {
                        VescProtocolMessage.RefloatTelemetry(decodeRealtimeData(payload))
                    } else if (
                        payload.size >= 35 &&
                        payload[1].toInt() and 0xff == 101 &&
                        payload[2].toInt() and 0xff == 10
                    ) {
                        VescProtocolMessage.RefloatTelemetry(decodeRefloatTelemetry(payload))
                    } else {
                        null
                    }
                }
                else -> null
            }
        }

    private fun decodeRefloatVersion(payload: ByteArray): String {
        val responseVersion = payload[3].toInt() and 0xff
        if (responseVersion > 10) return "${responseVersion / 10}.${responseVersion % 10}"
        require(responseVersion == 2 && payload.size >= 61) {
            "Truncated Refloat INFO v2 response"
        }
        return "${payload[25].toInt() and 0xff}.${payload[26].toInt() and 0xff}." +
            "${payload[27].toInt() and 0xff}"
    }

    private fun decodeRealtimeFields(payload: ByteArray): Pair<List<String>, List<String>> {
        var index = 3
        fun list(): List<String> {
            require(index < payload.size) { "Truncated Refloat field list" }
            val count = payload[index++].toInt() and 0xff
            return List(count) {
                require(index < payload.size) { "Truncated Refloat field ID" }
                val length = payload[index++].toInt() and 0xff
                require(index + length <= payload.size) { "Truncated Refloat field ID" }
                payload.copyOfRange(index, index + length).decodeToString().also {
                    index += length
                }
            }
        }
        return list() to list()
    }

    private fun decodeRealtimeData(payload: ByteArray): RefloatTelemetryRecord {
        require(realtimeFields.isNotEmpty()) { "Refloat telemetry IDs are unavailable" }
        val mask = payload[3].toInt() and 0xff
        val baseCount = realtimeFields.size
        val runtimeCount = if (mask and 0x1 != 0) realtimeRuntimeFields.size else 0
        require(payload.size >= 13 + (baseCount + runtimeCount) * 2) {
            "Truncated Refloat realtime response"
        }
        var index = 13
        val values = linkedMapOf<String, Double>()
        for (field in realtimeFields) {
            values[field] = decodeFloat16(payload, index)
            index += 2
        }
        if (runtimeCount > 0) {
            for (field in realtimeRuntimeFields) {
                values[field] = decodeFloat16(payload, index)
                index += 2
            }
        }
        val state = payload[9].toInt() and 0x03
        val footpad = (payload[10].toInt() ushr 6) and 0x03
        return RefloatTelemetryRecord(
            rideState = if (state == 3) RideState.RIDING else RideState.IDLE,
            boardState = state,
            footpadState = footpad,
            packVoltageV = values["motor.batt_voltage"] ?: 0.0,
            dutyCycle = values["motor.duty_cycle"] ?: 0.0,
            controllerTempC = values["motor.mosfet_temp"],
            motorTempC = values["motor.motor_temp"],
            motorCurrentA = values["motor.current"] ?: 0.0,
            batteryCurrentA = values["motor.batt_current"] ?: 0.0,
            erpm = (values["motor.erpm"] ?: 0.0).toInt(),
            speedKph = (values["motor.speed"] ?: 0.0) * 3.6,
        )
    }

    private fun decodeFloat16(payload: ByteArray, index: Int): Double {
        val bits =
            ((payload[index].toInt() and 0xff) shl 8) or
                (payload[index + 1].toInt() and 0xff)
        val sign = if (bits and 0x8000 == 0) 1.0 else -1.0
        val exponent = bits ushr 10 and 0x1f
        val fraction = bits and 0x03ff
        return if (exponent == 0) {
            sign * fraction * Math.scalb(1.0, -24)
        } else {
            sign * (1.0 + fraction / 1024.0) * Math.scalb(1.0, exponent - 15)
        }
    }

    private fun decodeRefloatTelemetry(payload: ByteArray): RefloatTelemetryRecord {
        val detailLevel = payload[3].toInt() and 0xff
        require(detailLevel in 1..3) { "Unsupported Refloat detail level" }
        require(payload.size >= if (detailLevel >= 3) 55 else if (detailLevel >= 2) 42 else 35) {
            "Truncated Refloat telemetry response"
        }
        fun unsigned(index: Int) = payload[index].toInt() and 0xff
        fun signed16(index: Int): Int =
            ((unsigned(index) shl 8) or unsigned(index + 1)).toShort().toInt()
        val state = unsigned(10) and 0x0f
        val switchBits = unsigned(11) and 0x0f
        val adc1 = unsigned(12) / 50.0
        val adc2 = unsigned(13) / 50.0
        val footpad = when (switchBits) {
            2 -> 3
            1 -> if (adc1 > adc2) 1 else 2
            else -> 0
        }
        return RefloatTelemetryRecord(
            rideState = if (state in 1..5) RideState.RIDING else RideState.IDLE,
            boardState = state,
            footpadState = footpad,
            packVoltageV = signed16(23) / 10.0,
            dutyCycle = (unsigned(33) - 128) / 100.0,
            controllerTempC = if (detailLevel >= 2) unsigned(39) / 2.0 else null,
            motorTempC = if (detailLevel >= 2) unsigned(40) / 2.0 else null,
            motorCurrentA = signed16(29) / 10.0,
            batteryCurrentA = signed16(31) / 10.0,
            erpm = signed16(25),
            speedKph = signed16(27) / 10.0 * 3.6,
        )
    }
}
