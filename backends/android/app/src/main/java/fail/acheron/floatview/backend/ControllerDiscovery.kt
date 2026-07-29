package fail.acheron.floatview.backend

import fail.acheron.floatview.protocol.refloat.RefloatCompatibility
import fail.acheron.floatview.protocol.refloat.RefloatTelemetryFamily
import fail.acheron.floatview.protocol.vesc.ControllerRoute
import fail.acheron.floatview.protocol.vesc.VescCompatibility

object ControllerDiscoveryPolicy {
    const val MAXIMUM_CAN_IDS = 64
    const val MAXIMUM_CANDIDATES = 16

    fun normalizedCanIds(values: List<Int>): List<Int> =
        values.filter { it in 0..253 }.distinct().sorted()

    fun meaningfulUuid(value: ByteArray?): ByteArray? {
        if (
            value == null ||
            value.size != 12 ||
            value.all { it == 0.toByte() } ||
            value.all { it == 0xff.toByte() }
        ) {
            return null
        }
        return value.copyOf()
    }

    fun preferredCanId(valuesJson: String?): Int? {
        if (valuesJson == null) return null
        val raw = PREFERRED_CAN_ID.find(valuesJson)?.groupValues?.get(1) ?: return null
        val value = raw.toDoubleOrNull() ?: return null
        return value.toInt().takeIf { value.isFinite() && value % 1.0 == 0.0 && it in 0..253 }
    }

    fun version(value: FirmwareRecord): String =
        "${value.major}.${value.minor.toString().padStart(2, '0')}"

    fun samePublicIdentity(left: FirmwareRecord, right: FirmwareRecord): Boolean =
        left.major == right.major &&
            left.minor == right.minor &&
            left.hardwareName == right.hardwareName &&
            left.firmwareName == right.firmwareName &&
            left.hardwareType == right.hardwareType

    fun boundedDisplayValue(value: String?): String? = value?.take(80)

    private val PREFERRED_CAN_ID =
        Regex(""""preferredCanId"\s*:\s*(-?\d+(?:\.\d+)?)""")
}

sealed interface ControllerProbeIntent {
    data class Preferred(val canId: Int) : ControllerProbeIntent
    data class Hint(val canId: Int, val controllerUuid: ByteArray) : ControllerProbeIntent
    data object Enumerating : ControllerProbeIntent
}

data class ControllerContinuity(
    val hasActiveSession: Boolean,
    val route: ControllerRoute?,
    val controllerUuid: ByteArray?,
)

sealed interface ControllerDiscoveryEffect {
    data class ProbeFirmware(val route: ControllerRoute) : ControllerDiscoveryEffect
    data object PingCan : ControllerDiscoveryEffect
    data class ClearHintThen(val next: ControllerDiscoveryEffect) : ControllerDiscoveryEffect
    data class RequestRefloatInfo(
        val route: ControllerRoute,
        val rememberController: FirmwareRecord? = null,
    ) : ControllerDiscoveryEffect
    data object RequestTelemetryLayout : ControllerDiscoveryEffect
    data object Ready : ControllerDiscoveryEffect
    data object SelectingController : ControllerDiscoveryEffect
    data class Failed(val issue: ConnectionIssue) : ControllerDiscoveryEffect
    data object None : ControllerDiscoveryEffect
}

/**
 * Pure, generation-aware controller discovery state machine.
 *
 * ApplicationService executes the returned effects against Bluetooth and
 * persistence, while this owner retains all direct/CAN route decisions.
 */
class ControllerDiscovery {
    var firmware: FirmwareRecord? = null
        private set
    var transportFirmware: FirmwareRecord? = null
        private set
    var refloatPackageVersion: String? = null
        private set
    var refloatFamily: RefloatTelemetryFamily? = null
        private set
    var controllerRoute: ControllerRoute? = null
        private set
    var controllerCandidates: List<ControllerCandidate> = emptyList()
        private set
    var connectionIssue: ConnectionIssue? = null
        private set
    var started: Boolean = false
        private set
    var identificationGeneration: Long = 0
        private set
    var canProbeGeneration: Long = 0
        private set
    var activeCanProbeId: Int? = null
        private set

    private val probedControllers = linkedMapOf<Int, FirmwareRecord>()
    private var pendingCanIds = mutableListOf<Int>()
    private var probeIntent: ControllerProbeIntent? = null
    private var recoveryControllerUuid: ByteArray? = null
    private var continuity = ControllerContinuity(false, null, null)

    fun begin(): Pair<Long, ControllerDiscoveryEffect> {
        if (started) return identificationGeneration to ControllerDiscoveryEffect.None
        started = true
        identificationGeneration += 1
        return identificationGeneration to
            ControllerDiscoveryEffect.ProbeFirmware(ControllerRoute.Direct)
    }

    fun acceptTransport(
        value: FirmwareRecord,
        preferredCanId: Int?,
        routingHint: ControllerRoutingHint?,
    ): ControllerDiscoveryEffect {
        transportFirmware = value
        if (value.hardwareType == VESC_HARDWARE_TYPE) {
            controllerRoute = ControllerRoute.Direct
            firmware = value
            return if (VescCompatibility.supports(value)) {
                ControllerDiscoveryEffect.RequestRefloatInfo(ControllerRoute.Direct)
            } else {
                fail(
                    ConnectionIssueCode.UNSUPPORTED_CONTROLLER,
                    "Direct controller firmware ${ControllerDiscoveryPolicy.version(value)} is not supported.",
                )
            }
        }
        if (preferredCanId != null) {
            if (routingHint?.canId == preferredCanId) {
                recoveryControllerUuid =
                    ControllerDiscoveryPolicy.meaningfulUuid(routingHint.controllerUuid)
            }
            probeIntent = ControllerProbeIntent.Preferred(preferredCanId)
            return probe(preferredCanId)
        }
        val hintUuid = ControllerDiscoveryPolicy.meaningfulUuid(routingHint?.controllerUuid)
        if (routingHint != null && hintUuid != null) {
            recoveryControllerUuid = hintUuid
            probeIntent = ControllerProbeIntent.Hint(routingHint.canId, hintUuid)
            return probe(routingHint.canId)
        }
        return beginEnumeration()
    }

    fun acceptCanFirmware(
        value: FirmwareRecord,
        continuity: ControllerContinuity,
    ): ControllerDiscoveryEffect {
        this.continuity = continuity
        val canId = value.canId ?: return ControllerDiscoveryEffect.None
        if (activeCanProbeId != canId) return ControllerDiscoveryEffect.None
        activeCanProbeId = null
        return when (val intent = probeIntent) {
            is ControllerProbeIntent.Preferred -> {
                if (intent.canId != canId) {
                    ControllerDiscoveryEffect.None
                } else if (value.hardwareType != VESC_HARDWARE_TYPE) {
                    fail(
                        ConnectionIssueCode.PREFERRED_CONTROLLER_UNAVAILABLE,
                        "The configured CAN controller is unavailable.",
                    )
                } else {
                    val expectedUuid = recoveryControllerUuid
                    if (
                        expectedUuid != null &&
                        ControllerDiscoveryPolicy.meaningfulUuid(value.controllerUuid)
                            ?.contentEquals(expectedUuid) != true
                    ) {
                        fail(
                            ConnectionIssueCode.CONTROLLER_CHANGED,
                            "The configured CAN route now identifies a different controller.",
                        )
                    } else {
                        choose(value)
                    }
                }
            }
            is ControllerProbeIntent.Hint -> {
                if (intent.canId != canId) {
                    ControllerDiscoveryEffect.None
                } else {
                    val uuid = ControllerDiscoveryPolicy.meaningfulUuid(value.controllerUuid)
                    if (
                        value.hardwareType == VESC_HARDWARE_TYPE &&
                        uuid != null &&
                        uuid.contentEquals(intent.controllerUuid)
                    ) {
                        choose(value)
                    } else {
                        ControllerDiscoveryEffect.ClearHintThen(beginEnumeration())
                    }
                }
            }
            ControllerProbeIntent.Enumerating -> {
                if (value.hardwareType == VESC_HARDWARE_TYPE) {
                    probedControllers[canId] = value
                    if (probedControllers.size > ControllerDiscoveryPolicy.MAXIMUM_CANDIDATES) {
                        return fail(
                            ConnectionIssueCode.TOO_MANY_CONTROLLERS,
                            "More than ${ControllerDiscoveryPolicy.MAXIMUM_CANDIDATES} VESC controllers were found.",
                        )
                    }
                }
                nextCanProbe(continuity)
            }
            null -> ControllerDiscoveryEffect.None
        }
    }

    fun acceptCanIds(
        rawCanIds: List<Int>,
        continuity: ControllerContinuity,
    ): ControllerDiscoveryEffect {
        this.continuity = continuity
        if (probeIntent != ControllerProbeIntent.Enumerating) {
            return ControllerDiscoveryEffect.None
        }
        val normalized = ControllerDiscoveryPolicy.normalizedCanIds(rawCanIds)
        if (normalized.size > ControllerDiscoveryPolicy.MAXIMUM_CAN_IDS) {
            return fail(
                ConnectionIssueCode.TOO_MANY_CAN_DEVICES,
                "The CAN network reported more than ${ControllerDiscoveryPolicy.MAXIMUM_CAN_IDS} devices.",
            )
        }
        pendingCanIds = normalized.toMutableList()
        return nextCanProbe(continuity)
    }

    fun acceptRefloatInfo(version: String): ControllerDiscoveryEffect {
        val entry = RefloatCompatibility.entry(version)
            ?: return fail(
                ConnectionIssueCode.UNSUPPORTED_REFLOAT,
                "This controller's Refloat version is not supported.",
            )
        refloatPackageVersion = version
        refloatFamily = entry.family
        return if (entry.family == RefloatTelemetryFamily.DYNAMIC) {
            ControllerDiscoveryEffect.RequestTelemetryLayout
        } else {
            ControllerDiscoveryEffect.Ready
        }
    }

    fun acceptTelemetryLayout(): ControllerDiscoveryEffect = ControllerDiscoveryEffect.Ready

    fun selectController(canId: Int): ControllerDiscoveryEffect {
        val candidate = controllerCandidates.firstOrNull { it.canId == canId }
        val selected = probedControllers[canId]
        check(candidate?.vescSupported == true && selected != null) {
            "Controller selection is not valid for the current state"
        }
        return choose(selected)
    }

    fun identificationTimedOut(generation: Long): ControllerDiscoveryEffect {
        if (generation != identificationGeneration) return ControllerDiscoveryEffect.None
        return when (probeIntent) {
            is ControllerProbeIntent.Preferred -> fail(
                ConnectionIssueCode.PREFERRED_CONTROLLER_UNAVAILABLE,
                "The configured CAN controller did not respond.",
            )
            is ControllerProbeIntent.Hint -> {
                identificationGeneration += 1
                ControllerDiscoveryEffect.ClearHintThen(beginEnumeration())
            }
            else -> fail(
                ConnectionIssueCode.IDENTIFICATION_TIMEOUT,
                "Controller identification timed out.",
            )
        }
    }

    fun canProbeTimedOut(canId: Int, generation: Long): ControllerDiscoveryEffect {
        if (activeCanProbeId != canId || canProbeGeneration != generation) {
            return ControllerDiscoveryEffect.None
        }
        activeCanProbeId = null
        return when (probeIntent) {
            is ControllerProbeIntent.Preferred -> fail(
                ConnectionIssueCode.PREFERRED_CONTROLLER_UNAVAILABLE,
                "The configured CAN controller did not respond.",
            )
            is ControllerProbeIntent.Hint ->
                ControllerDiscoveryEffect.ClearHintThen(beginEnumeration())
            ControllerProbeIntent.Enumerating ->
                nextCanProbe(continuity)
            null -> ControllerDiscoveryEffect.None
        }
    }

    fun markFailure(code: ConnectionIssueCode, message: String): ControllerDiscoveryEffect =
        fail(code, message)

    fun clearIssue() {
        connectionIssue = null
    }

    fun reset() {
        firmware = null
        transportFirmware = null
        refloatPackageVersion = null
        refloatFamily = null
        pendingCanIds.clear()
        probedControllers.clear()
        probeIntent = null
        controllerCandidates = emptyList()
        controllerRoute = null
        connectionIssue = null
        recoveryControllerUuid = null
        continuity = ControllerContinuity(false, null, null)
        activeCanProbeId = null
        started = false
        identificationGeneration += 1
        canProbeGeneration += 1
    }

    private fun beginEnumeration(): ControllerDiscoveryEffect {
        probeIntent = ControllerProbeIntent.Enumerating
        pendingCanIds.clear()
        probedControllers.clear()
        activeCanProbeId = null
        return ControllerDiscoveryEffect.PingCan
    }

    private fun probe(canId: Int): ControllerDiscoveryEffect {
        activeCanProbeId = canId
        canProbeGeneration += 1
        return ControllerDiscoveryEffect.ProbeFirmware(ControllerRoute.Can(canId))
    }

    private fun nextCanProbe(continuity: ControllerContinuity): ControllerDiscoveryEffect {
        val canId = pendingCanIds.removeFirstOrNull()
        return if (canId == null) finishCandidates(continuity) else probe(canId)
    }

    private fun finishCandidates(continuity: ControllerContinuity): ControllerDiscoveryEffect {
        val records = probedControllers.values.sortedBy { it.canId ?: 255 }
        controllerCandidates = records.mapNotNull { value ->
            value.canId?.let { canId ->
                ControllerCandidate(
                    canId,
                    ControllerDiscoveryPolicy.boundedDisplayValue(value.hardwareName),
                    ControllerDiscoveryPolicy.boundedDisplayValue(value.firmwareName),
                    ControllerDiscoveryPolicy.version(value),
                    VescCompatibility.supports(value),
                )
            }
        }
        if (records.isEmpty()) {
            return fail(
                if (continuity.hasActiveSession) {
                    ConnectionIssueCode.CONTROLLER_CHANGED
                } else {
                    ConnectionIssueCode.NO_CONTROLLER_FOUND
                },
                if (continuity.hasActiveSession) {
                    "The controller from the active session was not found."
                } else {
                    "No VESC motor controller was found."
                },
            )
        }
        recoveryControllerUuid?.let { expected ->
            records.firstOrNull {
                ControllerDiscoveryPolicy.meaningfulUuid(it.controllerUuid)
                    ?.contentEquals(expected) == true
            }?.let { return choose(it) }
        }
        if (continuity.hasActiveSession) {
            if (continuity.controllerUuid != null) {
                return fail(
                    ConnectionIssueCode.CONTROLLER_CHANGED,
                    "The controller from the active session was not found.",
                )
            }
            val previousCanId = (continuity.route as? ControllerRoute.Can)?.canId
            records.firstOrNull {
                it.canId == previousCanId && VescCompatibility.supports(it)
            }?.let { return choose(it) }
            return fail(
                ConnectionIssueCode.CONTROLLER_CHANGED,
                "The controller from the active session was not found.",
            )
        }
        if (records.size == 1) {
            val only = records.single()
            if (!VescCompatibility.supports(only)) {
                controllerRoute = ControllerRoute.Can(requireNotNull(only.canId))
                firmware = only
                return fail(
                    ConnectionIssueCode.UNSUPPORTED_CONTROLLER,
                    "Controller firmware ${ControllerDiscoveryPolicy.version(only)} is not supported.",
                )
            }
            return choose(only)
        }
        return ControllerDiscoveryEffect.SelectingController
    }

    private fun choose(value: FirmwareRecord): ControllerDiscoveryEffect {
        val canId = value.canId ?: return ControllerDiscoveryEffect.None
        controllerRoute = ControllerRoute.Can(canId)
        firmware = value
        controllerCandidates = emptyList()
        if (!VescCompatibility.supports(value)) {
            return fail(
                ConnectionIssueCode.UNSUPPORTED_CONTROLLER,
                "Controller firmware ${ControllerDiscoveryPolicy.version(value)} is not supported.",
            )
        }
        return ControllerDiscoveryEffect.RequestRefloatInfo(
            ControllerRoute.Can(canId),
            rememberController = value,
        )
    }

    private fun fail(code: ConnectionIssueCode, message: String): ControllerDiscoveryEffect {
        val issue = ConnectionIssue(code, message)
        connectionIssue = issue
        return ControllerDiscoveryEffect.Failed(issue)
    }

    private companion object {
        const val VESC_HARDWARE_TYPE = 0
    }
}
