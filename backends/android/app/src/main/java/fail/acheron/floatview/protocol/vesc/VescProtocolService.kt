package fail.acheron.floatview.protocol

import fail.acheron.floatview.backend.FirmwareRecord
import fail.acheron.floatview.backend.BmsConnection
import fail.acheron.floatview.backend.BmsSnapshot
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
import java.nio.ByteBuffer
import java.nio.charset.CharacterCodingException
import java.nio.charset.CodingErrorAction
import java.time.Instant

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
    val remaining: Int get() = bytes.size - offset
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
        val signedValue = if (raw and 0x80000000L == 0L) value else -value
        if (!signedValue.isFinite() || kotlin.math.abs(signedValue) > Float.MAX_VALUE.toDouble()) {
            throw VescProtocolException.TruncatedPayload
        }
        return signedValue
    }
    fun skip(count: Int) {
        if (offset + count > bytes.size) throw VescProtocolException.TruncatedPayload
        offset += count
    }
    fun terminatedUtf8(maxContentBytes: Int): String {
        val start = offset
        val searchEnd = minOf(bytes.size, start + maxContentBytes + 1)
        var terminator = start
        while (terminator < searchEnd && bytes[terminator] != 0.toByte()) terminator++
        if (terminator == searchEnd) throw VescProtocolException.TruncatedPayload
        val decoder = Charsets.UTF_8.newDecoder()
            .onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT)
        val value = try {
            decoder.decode(ByteBuffer.wrap(bytes, start, terminator - start)).toString()
        } catch (_: CharacterCodingException) {
            throw VescProtocolException.TruncatedPayload
        }
        offset = terminator + 1
        return value
    }
}

object VescPayloadDecoder {
    private val batterySeriesCellOffsets = mapOf(
        0x2e43a161L to 462, // 6.02
        0x3f829cf7L to 452, // 6.05
        0x2efd0142L to 458, // 6.06
        0x57ad8f53L to 463, // 7.00
        0x9307410dL to 463, // 7.01
    )

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

    fun bmsValues(
        payload: ByteArray,
        expectedCanId: Int? = null,
        observedAt: Instant,
    ): BmsSnapshot {
        val reader = ByteReader(payload)
        val command = reader.u8()
        if (command != VescCommand.BMS_GET_VALUES.id) {
            throw VescProtocolException.UnsupportedCommand(command)
        }

        val totalVoltage = reader.i32(1_000_000.0)
        val chargeVoltage = reader.i32(1_000_000.0)
        val inputCurrent = reader.i32(1_000_000.0)
        val inputIcCurrent = reader.i32(1_000_000.0)
        val sessionAmpHours = reader.i32(1_000.0)
        val sessionWattHours = reader.i32(1_000.0)

        val cellCount = reader.u8()
        if (cellCount > 50) throw VescProtocolException.TruncatedPayload
        val cellVoltages = List(cellCount) { reader.i16(1_000.0) }
        val balancing = List(cellCount) {
            when (reader.u8()) {
                0 -> false
                1 -> true
                else -> throw VescProtocolException.TruncatedPayload
            }
        }

        val temperatureCount = reader.u8()
        if (temperatureCount > 50) throw VescProtocolException.TruncatedPayload
        val temperatures = List(temperatureCount) { reader.i16(100.0) }
        val icTemperature = reader.i16(100.0)
        val humiditySensorTemperature = reader.i16(100.0)
        val humidity = reader.i16(100.0)
        val highestCellTemperature = reader.i16(100.0)
        val stateOfCharge = reader.i16(1_000.0)
        val stateOfHealth = reader.i16(1_000.0)
        val reportedCanId = reader.u8()
        if (reportedCanId > 253 || expectedCanId != null && reportedCanId != expectedCanId) {
            throw VescProtocolException.TruncatedPayload
        }
        val lifetimeChargeAmpHours = reader.float32Auto()
        val lifetimeChargeWattHours = reader.float32Auto()
        val lifetimeDischargeAmpHours = reader.float32Auto()
        val lifetimeDischargeWattHours = reader.float32Auto()
        val pressure = reader.i16(0.1)

        val dataVersion: Int?
        val status: String?
        if (reader.remaining == 0) {
            dataVersion = null
            status = null
        } else {
            dataVersion = reader.u8()
            status = reader.terminatedUtf8(40)
            if (reader.remaining != 0) throw VescProtocolException.TruncatedPayload
        }

        val numericValues = listOf(
            totalVoltage, chargeVoltage, inputCurrent, inputIcCurrent,
            sessionAmpHours, sessionWattHours, icTemperature,
            humiditySensorTemperature, humidity, highestCellTemperature,
            stateOfCharge, stateOfHealth, lifetimeChargeAmpHours,
            lifetimeChargeWattHours, lifetimeDischargeAmpHours,
            lifetimeDischargeWattHours, pressure,
        ) + cellVoltages + temperatures
        if (numericValues.any { !it.isFinite() }) throw VescProtocolException.TruncatedPayload

        return BmsSnapshot(
            observedAt = observedAt,
            totalVoltageV = totalVoltage,
            chargeVoltageV = chargeVoltage,
            inputCurrentA = inputCurrent,
            inputIcCurrentA = inputIcCurrent,
            sessionAmpHoursAh = sessionAmpHours,
            sessionWattHoursWh = sessionWattHours,
            cellVoltagesV = cellVoltages,
            balancing = balancing,
            temperaturesC = temperatures,
            icTemperatureC = icTemperature,
            humiditySensorTemperatureC = humiditySensorTemperature,
            humidityPercent = humidity,
            highestCellTemperatureC = highestCellTemperature,
            stateOfCharge = stateOfCharge,
            stateOfHealth = stateOfHealth,
            reportedCanId = reportedCanId,
            lifetimeChargeAmpHoursAh = lifetimeChargeAmpHours,
            lifetimeChargeWattHoursWh = lifetimeChargeWattHours,
            lifetimeDischargeAmpHoursAh = lifetimeDischargeAmpHours,
            lifetimeDischargeWattHoursWh = lifetimeDischargeWattHours,
            pressurePa = pressure,
            dataVersion = dataVersion,
            status = status,
        )
    }

    fun currentLimits(payload: ByteArray): ControllerCurrentLimits {
        val reader = ByteReader(payload)
        val command = reader.u8()
        if (command != VescCommand.GET_MOTOR_CONFIGURATION.id) {
            throw VescProtocolException.UnsupportedCommand(command)
        }
        var signature = 0L
        repeat(4) { signature = (signature shl 8) or reader.u8().toLong() }
        reader.skip(4) // Four enum fields.
        val batterySeriesCells = batterySeriesCellOffsets[signature]?.let { offset ->
            payload.getOrNull(offset)?.toInt()?.and(0xff)?.takeIf { it > 0 }
        }
        val value = ControllerCurrentLimits(
            reader.float32Auto(),
            reader.float32Auto(),
            reader.float32Auto(),
            reader.float32Auto(),
            batterySeriesCells,
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
    override val supportsBmsTelemetry: Boolean = true
    private var stream = VescFrameStreamDecoder()
    private val dynamicTelemetry = RefloatDynamicTelemetryDecoder()

    @Synchronized
    override fun reset() {
        stream = VescFrameStreamDecoder()
        dynamicTelemetry.reset()
    }

    @Synchronized
    override fun firmwareRequest(route: ControllerRoute): ByteArray {
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

    override fun bmsValuesRequest(connection: BmsConnection): ByteArray {
        val route = when (connection) {
            BmsConnection.Direct -> ControllerRoute.Direct
            is BmsConnection.Can -> ControllerRoute.Can(connection.canId)
        }
        return VescFrameCodec.encode(
            VescCommandRouter.route(byteArrayOf(VescCommand.BMS_GET_VALUES.byte), route),
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
                            null,
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
                VescCommand.BMS_GET_VALUES.id -> VescProtocolMessage.BmsValuesPayload(payload)
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
