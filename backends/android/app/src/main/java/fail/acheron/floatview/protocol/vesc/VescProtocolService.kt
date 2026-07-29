package fail.acheron.floatview.protocol

import fail.acheron.floatview.backend.FirmwareRecord
import fail.acheron.floatview.backend.VescProtocolMessage
import fail.acheron.floatview.backend.VescProtocolPort
import fail.acheron.floatview.protocol.refloat.RefloatDynamicTelemetryDecoder
import fail.acheron.floatview.protocol.refloat.RefloatInfoDecoder
import fail.acheron.floatview.protocol.refloat.RefloatLegacyTelemetryDecoder
import fail.acheron.floatview.protocol.vesc.ControllerRoute
import fail.acheron.floatview.protocol.vesc.VescCommandRouter
import fail.acheron.floatview.protocol.vesc.VescFirmwareIdentityDecoder

data class FirmwareInfo(
    val major: Int,
    val minor: Int,
    val hardwareName: String,
    val firmwareName: String? = null,
    val hardwareType: Int = 0,
    val controllerUuid: ByteArray? = null,
) {
    override fun equals(other: Any?): Boolean =
        other is FirmwareInfo &&
            major == other.major &&
            minor == other.minor &&
            hardwareName == other.hardwareName &&
            firmwareName == other.firmwareName &&
            hardwareType == other.hardwareType &&
            (
                controllerUuid == null && other.controllerUuid == null ||
                    controllerUuid != null &&
                    other.controllerUuid != null &&
                    controllerUuid.contentEquals(other.controllerUuid)
                )

    override fun hashCode(): Int {
        var result = major
        result = 31 * result + minor
        result = 31 * result + hardwareName.hashCode()
        result = 31 * result + (firmwareName?.hashCode() ?: 0)
        result = 31 * result + hardwareType
        result = 31 * result + (controllerUuid?.contentHashCode() ?: 0)
        return result
    }
}

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
        return VescFirmwareIdentityDecoder.decode(payload)
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

}

class VescProtocolService : VescProtocolPort {
    private var stream = VescFrameStreamDecoder()
    private var pendingFirmwareCanId: Int? = null
    private val dynamicTelemetry = RefloatDynamicTelemetryDecoder()

    @Synchronized
    override fun reset() {
        stream = VescFrameStreamDecoder()
        pendingFirmwareCanId = null
        dynamicTelemetry.reset()
    }

    @Synchronized
    override fun firmwareRequest(route: ControllerRoute): ByteArray {
        pendingFirmwareCanId = (route as? ControllerRoute.Can)?.canId
        val payload = VescCommandRouter.route(byteArrayOf(0), route)
        return VescFrameCodec.encode(payload)
    }

    override fun canPingRequest(): ByteArray = VescFrameCodec.encode(byteArrayOf(62))

    override fun refloatInfoRequest(route: ControllerRoute): ByteArray {
        val command = byteArrayOf(36, 101, 0, 2, 0)
        val payload = VescCommandRouter.route(command, route)
        return VescFrameCodec.encode(payload)
    }

    override fun refloatTelemetryIdsRequest(route: ControllerRoute): ByteArray {
        return VescFrameCodec.encode(
            VescCommandRouter.route(byteArrayOf(36, 101, 32), route),
        )
    }

    override fun refloatTelemetryRequest(route: ControllerRoute): ByteArray {
        return VescFrameCodec.encode(
            VescCommandRouter.route(byteArrayOf(36, 101, 31), route),
        )
    }

    override fun refloatLegacyTelemetryRequest(route: ControllerRoute): ByteArray {
        return VescFrameCodec.encode(
            VescCommandRouter.route(byteArrayOf(36, 101, 10), route),
        )
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
                            info.controllerUuid,
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
                        VescProtocolMessage.RefloatInfo(
                            RefloatInfoDecoder.decodeVersion(payload),
                        )
                    } else if (
                        payload.size >= 5 &&
                        payload[1].toInt() and 0xff == 101 &&
                        payload[2].toInt() and 0xff == 32
                    ) {
                        val (fields, runtimeFields) = dynamicTelemetry.decodeFields(payload)
                        VescProtocolMessage.RefloatTelemetryLayout(fields, runtimeFields)
                    } else if (
                        payload.size >= 13 &&
                        payload[1].toInt() and 0xff == 101 &&
                        payload[2].toInt() and 0xff == 31
                    ) {
                        VescProtocolMessage.RefloatTelemetry(dynamicTelemetry.decodeData(payload))
                    } else if (
                        payload.size >= 35 &&
                        payload[1].toInt() and 0xff == 101 &&
                        payload[2].toInt() and 0xff == 10
                    ) {
                        VescProtocolMessage.RefloatTelemetry(
                            RefloatLegacyTelemetryDecoder.decode(payload),
                        )
                    } else {
                        null
                    }
                }
                else -> null
            }
        }

}
