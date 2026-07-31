package fail.acheron.floatview.protocol

import fail.acheron.floatview.backend.FirmwareRecord
import fail.acheron.floatview.backend.ControllerCurrentLimits
import fail.acheron.floatview.backend.VescProtocolMessage
import fail.acheron.floatview.backend.VescProtocolPort
import fail.acheron.floatview.protocol.refloat.RefloatDynamicTelemetryDecoder
import fail.acheron.floatview.protocol.refloat.RefloatInfoDecoder
import fail.acheron.floatview.protocol.refloat.RefloatLegacyTelemetryDecoder
import fail.acheron.floatview.protocol.refloat.RefloatCommand
import fail.acheron.floatview.protocol.refloat.RefloatWire
import fail.acheron.floatview.protocol.vesc.ControllerRoute
import fail.acheron.floatview.protocol.vesc.VescCommand
import fail.acheron.floatview.protocol.vesc.VescCommandRouter
import fail.acheron.floatview.protocol.vesc.VescFirmwareIdentityDecoder

data class FirmwareInfo(
    val major: Int,
    val minor: Int,
    val hardwareName: String,
    val firmwareName: String? = null,
    val hardwareType: Int = 0,
    val controllerUuid: ByteArray? = null,
    val testVersion: Int = 0,
) {
    override fun equals(other: Any?): Boolean =
        other is FirmwareInfo &&
            major == other.major &&
            minor == other.minor &&
            testVersion == other.testVersion &&
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
        result = 31 * result + testVersion
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
    fun float32Auto(): Double {
        var raw = 0L
        repeat(4) { raw = (raw shl 8) or u8().toLong() }
        val exponentBits = ((raw shr 23) and 0xff).toInt()
        val significandBits = raw and 0x7fffff
        if (exponentBits == 0 && significandBits == 0L) return 0.0
        val significand = significandBits.toDouble() / 16_777_216.0 + 0.5
        val value = Math.scalb(significand, exponentBits - 126)
        return if (raw and 0x80000000L == 0L) value else -value
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
        if (command != VescCommand.GET_REALTIME_VALUES.id) {
            throw VescProtocolException.UnsupportedCommand(command)
        }
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

    fun currentLimits(payload: ByteArray): ControllerCurrentLimits {
        val reader = ByteReader(payload)
        val command = reader.u8()
        if (command != VescCommand.GET_MOTOR_CONFIGURATION.id) {
            throw VescProtocolException.UnsupportedCommand(command)
        }
        reader.skip(8) // MCCONF signature and four enum fields.
        val value = ControllerCurrentLimits(
            reader.float32Auto(),
            reader.float32Auto(),
            reader.float32Auto(),
            reader.float32Auto(),
        )
        val limits = listOf(
            value.motorCurrentMaxA,
            value.motorCurrentMinA,
            value.batteryCurrentMaxA,
            value.batteryCurrentMinA,
        )
        if (limits.any { !it.isFinite() || kotlin.math.abs(it) <= 0.0 }) {
            throw VescProtocolException.TruncatedPayload
        }
        return value
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
        val payload = VescCommandRouter.route(
            byteArrayOf(VescCommand.GET_FIRMWARE_VERSION.byte),
            route,
        )
        return VescFrameCodec.encode(payload)
    }

    override fun canPingRequest(): ByteArray =
        VescFrameCodec.encode(byteArrayOf(VescCommand.PING_CAN.byte))

    override fun motorConfigurationRequest(route: ControllerRoute): ByteArray =
        VescFrameCodec.encode(
            VescCommandRouter.route(byteArrayOf(VescCommand.GET_MOTOR_CONFIGURATION.byte), route),
        )

    override fun refloatInfoRequest(route: ControllerRoute): ByteArray {
        val command = RefloatWire.request(
            RefloatCommand.INFO,
            RefloatWire.INFO_PROTOCOL_VERSION,
            RefloatWire.INFO_FLAGS,
        )
        val payload = VescCommandRouter.route(command, route)
        return VescFrameCodec.encode(payload)
    }

    override fun refloatTelemetryIdsRequest(route: ControllerRoute): ByteArray {
        return VescFrameCodec.encode(
            VescCommandRouter.route(
                RefloatWire.request(RefloatCommand.REALTIME_DATA_INTERNAL_IDS),
                route,
            ),
        )
    }

    override fun refloatTelemetryRequest(route: ControllerRoute): ByteArray {
        return VescFrameCodec.encode(
            VescCommandRouter.route(
                RefloatWire.request(RefloatCommand.REALTIME_DATA_INTERNAL),
                route,
            ),
        )
    }

    override fun refloatLegacyTelemetryRequest(route: ControllerRoute): ByteArray {
        return VescFrameCodec.encode(
            VescCommandRouter.route(
                RefloatWire.request(
                    RefloatCommand.GET_ALL_DATA,
                    RefloatWire.LEGACY_TELEMETRY_DETAIL_LEVEL,
                ),
                route,
            ),
        )
    }

    override fun refloatCompactTelemetryRequest(route: ControllerRoute): ByteArray {
        return VescFrameCodec.encode(
            VescCommandRouter.route(
                RefloatWire.request(
                    RefloatCommand.GET_ALL_DATA,
                    RefloatWire.LEGACY_TELEMETRY_DETAIL_LEVEL,
                ),
                route,
            ),
        )
    }

    @Synchronized
    override fun receive(bytes: ByteArray): List<VescProtocolMessage> =
        stream.append(bytes).mapNotNull { payload ->
            when (payload.firstOrNull()?.toInt()?.and(0xff)) {
                VescCommand.GET_FIRMWARE_VERSION.id -> {
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
                            info.testVersion,
                        ),
                    )
                }
                VescCommand.GET_MOTOR_CONFIGURATION.id ->
                    VescProtocolMessage.CurrentLimits(VescPayloadDecoder.currentLimits(payload))
                VescCommand.PING_CAN.id -> VescProtocolMessage.CanDevices(
                    payload.drop(1).map { it.toInt() and 0xff }.distinct().sorted(),
                )
                VescCommand.CUSTOM_APP_DATA.id -> {
                    val command = RefloatWire.command(payload) ?: return@mapNotNull null
                    if (
                        payload.size >= 4 &&
                        command == RefloatCommand.INFO
                    ) {
                        VescProtocolMessage.RefloatInfo(
                            RefloatInfoDecoder.decodeVersion(payload),
                        )
                    } else if (
                        payload.size >= 5 &&
                        command == RefloatCommand.REALTIME_DATA_INTERNAL_IDS
                    ) {
                        val (fields, runtimeFields) = dynamicTelemetry.decodeFields(payload)
                        VescProtocolMessage.RefloatTelemetryLayout(fields, runtimeFields)
                    } else if (
                        payload.size >= 13 &&
                        command == RefloatCommand.REALTIME_DATA_INTERNAL
                    ) {
                        VescProtocolMessage.RefloatTelemetry(dynamicTelemetry.decodeData(payload))
                    } else if (
                        payload.size >= 4 &&
                        command == RefloatCommand.GET_ALL_DATA
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
