package fail.acheron.floatview.protocol

import fail.acheron.floatview.application.FirmwareRecord
import fail.acheron.floatview.application.BmsConnection
import fail.acheron.floatview.application.BmsSnapshot
import fail.acheron.floatview.application.ControllerCurrentLimits
import fail.acheron.floatview.application.VescProtocolMessage
import fail.acheron.floatview.application.VescProtocolPort
import fail.acheron.floatview.protocol.refloat.RefloatDynamicTelemetryDecoder
import fail.acheron.floatview.protocol.refloat.RefloatCompatibility
import fail.acheron.floatview.protocol.refloat.RefloatLegacyTelemetryDecoder
import fail.acheron.floatview.protocol.refloat.RefloatCommand
import fail.acheron.floatview.protocol.refloat.RefloatWire
import fail.acheron.floatview.protocol.refloat.RefloatBooleanControl
import fail.acheron.floatview.protocol.refloat.RefloatControlCodec
import fail.acheron.floatview.protocol.refloat.RefloatControlProfile
import fail.acheron.floatview.protocol.refloat.RefloatLightingProfile
import fail.acheron.floatview.protocol.refloat.RefloatLightingState
import fail.acheron.floatview.protocol.vesc.ControllerRoute
import fail.acheron.floatview.protocol.vesc.VescCommand
import fail.acheron.floatview.protocol.vesc.VescBmsValuesDecoder
import fail.acheron.floatview.protocol.vesc.VescCommandRouter
import fail.acheron.floatview.protocol.vesc.VescControllerDataDecoder
import fail.acheron.floatview.protocol.vesc.VescCustomConfigurationCodec
import fail.acheron.floatview.protocol.vesc.VescFirmwareIdentityDecoder
import java.time.Instant

data class FirmwareInfo(
    val major: Int,
    val minor: Int,
    val hardwareName: String,
    val firmwareName: String? = null,
    val hardwareType: Int = 0,
    val controllerUuid: ByteArray? = null,
    val testVersion: Int = 0,
    val customConfigurationCount: Int = 0,
    val hardwareConfigurationCrc: Long? = null,
) {
    override fun equals(other: Any?): Boolean =
        other is FirmwareInfo &&
            major == other.major &&
            minor == other.minor &&
            testVersion == other.testVersion &&
            hardwareName == other.hardwareName &&
            firmwareName == other.firmwareName &&
            hardwareType == other.hardwareType &&
            customConfigurationCount == other.customConfigurationCount &&
            hardwareConfigurationCrc == other.hardwareConfigurationCrc &&
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
        result = 31 * result + customConfigurationCount
        result = 31 * result + (hardwareConfigurationCrc?.hashCode() ?: 0)
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
    val fault: VescFault,
)

data class VescFault(val code: Int, val kind: Kind) {
    enum class Kind(val stableName: String) {
        NONE("none"),
        OVER_VOLTAGE("over_voltage"),
        UNKNOWN("unknown"),
    }

}

object VescPayloadDecoder {
    fun firmware(payload: ByteArray): FirmwareInfo {
        return VescFirmwareIdentityDecoder.decode(payload)
    }

    fun realtime(payload: ByteArray): RealtimeValues = VescControllerDataDecoder.realtime(payload)

    fun bmsValues(
        payload: ByteArray,
        expectedCanId: Int? = null,
        observedAt: Instant,
    ): BmsSnapshot = VescBmsValuesDecoder.decode(payload, expectedCanId, observedAt)

    fun currentLimits(payload: ByteArray): ControllerCurrentLimits =
        VescControllerDataDecoder.currentLimits(payload)

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

    override fun controllerShutdownRequest(route: ControllerRoute): ByteArray =
        VescFrameCodec.encode(
            VescCommandRouter.route(byteArrayOf(VescCommand.SHUTDOWN.byte, 0, 0), route),
        )

    override fun motorConfigurationRequest(route: ControllerRoute): ByteArray =
        VescFrameCodec.encode(
            VescCommandRouter.route(byteArrayOf(VescCommand.GET_MOTOR_CONFIGURATION.byte), route),
        )

    override fun refloatInfoRequest(route: ControllerRoute): ByteArray {
        val command = RefloatCompatibility.infoRequest()
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

    override fun customConfigurationSchemaRequest(route: ControllerRoute, length: Int, offset: Int): ByteArray =
        VescFrameCodec.encode(VescCommandRouter.route(VescCustomConfigurationCodec.schemaRequest(length = length, offset = offset), route))

    override fun customConfigurationValuesRequest(route: ControllerRoute, defaults: Boolean): ByteArray =
        VescFrameCodec.encode(VescCommandRouter.route(VescCustomConfigurationCodec.valuesRequest(defaults = defaults), route))

    override fun customConfigurationWriteRequest(route: ControllerRoute, values: ByteArray): ByteArray =
        VescFrameCodec.encode(VescCommandRouter.route(VescCustomConfigurationCodec.writeRequest(values = values), route))

    fun refloatPackageControlRequest(route: ControllerRoute, enabled: Boolean): ByteArray =
        routedRefloatControl(RefloatControlCodec.packageRequest(enabled), route)

    fun refloatFlywheelControlRequest(route: ControllerRoute, enabled: Boolean): ByteArray =
        routedRefloatControl(RefloatControlCodec.flywheelRequest(enabled), route)

    fun refloatLightingControlRequest(
        route: ControllerRoute,
        profile: RefloatControlProfile,
        control: RefloatBooleanControl?,
        enabled: Boolean = false,
        current: RefloatLightingState,
    ): ByteArray = routedRefloatControl(
        RefloatControlCodec.lightingRequest(profile, control, enabled, current),
        route,
    )

    fun refloatFatalClearRequest(route: ControllerRoute, profile: RefloatControlProfile): ByteArray =
        routedRefloatControl(RefloatControlCodec.fatalClearRequest(profile), route)

    private fun routedRefloatControl(command: ByteArray, route: ControllerRoute): ByteArray =
        VescFrameCodec.encode(VescCommandRouter.route(command, route))

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
                            info.customConfigurationCount,
                            info.hardwareConfigurationCrc,
                        ),
                    )
                }
                VescCommand.GET_MOTOR_CONFIGURATION.id ->
                    VescProtocolMessage.CurrentLimits(VescPayloadDecoder.currentLimits(payload))
                VescCommand.PING_CAN.id -> VescProtocolMessage.CanDevices(
                    payload.drop(1).map { it.toInt() and 0xff }.distinct().sorted(),
                )
                VescCommand.BMS_GET_VALUES.id -> VescProtocolMessage.BmsValuesPayload(payload)
                VescCommand.GET_CUSTOM_CONFIGURATION_XML.id,
                VescCommand.GET_CUSTOM_CONFIGURATION.id,
                VescCommand.GET_CUSTOM_CONFIGURATION_DEFAULT.id,
                VescCommand.SET_CUSTOM_CONFIGURATION.id ->
                    VescProtocolMessage.CustomConfiguration(VescCustomConfigurationCodec.decode(payload))
                VescCommand.CUSTOM_APP_DATA.id -> {
                    val command = RefloatWire.command(payload) ?: return@mapNotNull null
                    if (
                        payload.size >= 4 &&
                        command == RefloatCommand.INFO
                    ) {
                        VescProtocolMessage.RefloatInfo(
                            RefloatCompatibility.decodeInfo(payload),
                        )
                    } else if (
                        payload.size >= 5 &&
                        command == RefloatCommand.REALTIME_DATA_INTERNAL_IDS
                    ) {
                        val layout = dynamicTelemetry.decodeFieldsWithRecord(payload)
                        val supported = RefloatCompatibility.admits(layout.record)
                        if (!supported) dynamicTelemetry.reset()
                        VescProtocolMessage.RefloatTelemetryLayout(
                            layout.fields,
                            layout.runtimeFields,
                            supported,
                        )
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
                    } else if (command == RefloatCommand.LEGACY_LIGHTS) {
                        val profile = RefloatControlProfile(RefloatLightingProfile.LEGACY_COMMAND_202, false)
                        VescProtocolMessage.RefloatLighting(
                            command,
                            RefloatControlCodec.decodeLightingResponse(payload, profile),
                        )
                    } else if (command == RefloatCommand.STABLE_LIGHTS) {
                        val profile = RefloatControlProfile(RefloatLightingProfile.STABLE_COMMAND_20, true)
                        VescProtocolMessage.RefloatLighting(
                            command,
                            RefloatControlCodec.decodeLightingResponse(payload, profile),
                        )
                    } else {
                        null
                    }
                }
                else -> null
            }
        }

}
