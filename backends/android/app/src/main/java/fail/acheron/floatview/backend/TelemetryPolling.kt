package fail.acheron.floatview.backend

import fail.acheron.floatview.protocol.refloat.RefloatPolling
import fail.acheron.floatview.protocol.refloat.RefloatCommand
import fail.acheron.floatview.protocol.refloat.RefloatTelemetryFamily
import fail.acheron.floatview.protocol.vesc.ControllerRoute
import fail.acheron.floatview.protocol.vesc.VescCommand
import java.time.Instant

/**
 * Owns route-aware layout negotiation, one serialized poll stream, compact
 * caching, and normalized sample sequencing.
 */
class TelemetryPolling(
    private val clock: ClockPort,
    private val scheduler: SchedulerPort?,
    private val protocol: VescProtocolPort?,
    private val submit: (
        ByteArray,
        VescExpectedResponse,
        VescRequestPriority,
        VescRequestContext,
        Long,
        (VescRequestCompletion) -> Unit,
    ) -> Unit,
    private val onSample: (TelemetrySample) -> Unit,
    private val onFailure: (String) -> Unit,
    private val onDiagnostic: (Any) -> Unit,
) {
    private enum class PendingRequest { REALTIME, COMPACT, LEGACY }

    private var layoutTimer: CancellationPort? = null
    private var layoutAttempts = 0
    private var pollingTimer: CancellationPort? = null
    private var pollingIntervalMs: Long? = null
    private var startedAtMs: Long? = null
    private var sequence = 0L
    private var previousRideState = RideState.UNKNOWN
    private var postRideProbeUntilMs: Long? = null
    private var deviceId: DeviceId? = null
    private var route: ControllerRoute? = null
    private var family = RefloatTelemetryFamily.DYNAMIC
    private var idleIntervalMs = 1_000L
    private var postRideTailMs = 5_000L
    private var pendingRequest: PendingRequest? = null
    private var pendingSinceMs: Long? = null
    private var nextCompactDueMs: Long? = null
    private var latestCompact: Pair<RefloatTelemetryRecord, Instant>? = null
    private var active = false

    fun resetSession() {
        stop()
        cancelLayout()
        startedAtMs = null
        sequence = 0
        deviceId = null
        route = null
        family = RefloatTelemetryFamily.DYNAMIC
    }

    fun markSessionStarted() {
        startedAtMs = clock.monotonicMillis()
    }

    fun beginLayout(deviceId: DeviceId, route: ControllerRoute) {
        if (layoutTimer != null) return
        this.deviceId = deviceId
        this.route = route
        layoutAttempts = 0
        sendLayoutRequest()
        layoutTimer = scheduler?.scheduleRepeating(
            LAYOUT_RETRY_INTERVAL_MS,
            ::retryLayout,
        )
    }

    fun layoutEstablished() {
        cancelLayout()
    }

    fun start(
        deviceId: DeviceId,
        route: ControllerRoute,
        family: RefloatTelemetryFamily,
        idleIntervalMs: Long,
        postRideTailMs: Long,
    ) {
        if (scheduler == null || protocol == null) return
        this.deviceId = deviceId
        this.route = route
        this.family = family
        this.idleIntervalMs = idleIntervalMs
        this.postRideTailMs = postRideTailMs
        pendingRequest = null
        pendingSinceMs = null
        latestCompact = null
        nextCompactDueMs =
            if (family == RefloatTelemetryFamily.DYNAMIC) clock.monotonicMillis() + COMPACT_INTERVAL_MS
            else null
        active = true
        restart(if (previousRideState == RideState.RIDING) RIDING_INTERVAL_MS else idleIntervalMs)
    }

    fun updateSettings(idleIntervalMs: Long, postRideTailMs: Long) {
        this.idleIntervalMs = idleIntervalMs
        this.postRideTailMs = postRideTailMs
        if (active) {
            restart(
                if (previousRideState == RideState.RIDING) {
                    RIDING_INTERVAL_MS
                } else {
                    idleIntervalMs
                },
            )
        }
    }

    fun refresh() = requestSample()

    @Synchronized
    fun receive(value: RefloatTelemetryRecord) {
        if (!active || !responseMatchesPending(value)) return
        pendingRequest = null
        pendingSinceMs = null
        val observedAt = clock.now()
        if (
            family == RefloatTelemetryFamily.DYNAMIC &&
            value.kind == RefloatTelemetryKind.COMPACT
        ) {
            latestCompact = value to observedAt
            return
        }
        val nowMs = clock.monotonicMillis()
        if (startedAtMs == null) startedAtMs = nowMs
        val sample = normalizedSample(
            value,
            ++sequence,
            observedAt,
            maxOf(0, nowMs - requireNotNull(startedAtMs)),
        )
        onSample(sample)
        if (value.rideState == RideState.RIDING) {
            postRideProbeUntilMs = null
        } else if (previousRideState == RideState.RIDING) {
            postRideProbeUntilMs = nowMs + postRideTailMs
        }
        val inPostRideTail = postRideProbeUntilMs?.let { nowMs <= it } ?: false
        previousRideState = value.rideState
        val target =
            if (value.rideState == RideState.RIDING || inPostRideTail) {
                RIDING_INTERVAL_MS
            } else {
                idleIntervalMs
            }
        if (target != pollingIntervalMs) restart(target)
    }

    fun stop() {
        active = false
        pollingTimer?.cancel()
        pollingTimer = null
        pollingIntervalMs = null
        previousRideState = RideState.UNKNOWN
        postRideProbeUntilMs = null
        pendingRequest = null
        pendingSinceMs = null
        nextCompactDueMs = null
        latestCompact = null
    }

    fun cancelLayout() {
        layoutTimer?.cancel()
        layoutTimer = null
        layoutAttempts = 0
    }

    @Synchronized
    private fun retryLayout() {
        if (layoutTimer == null) return
        if (layoutAttempts >= MAX_LAYOUT_ATTEMPTS) {
            cancelLayout()
            onFailure("refloat_telemetry_layout_timeout")
            return
        }
        sendLayoutRequest()
    }

    private fun sendLayoutRequest() {
        if (deviceId == null) return
        val selectedRoute = route ?: return
        val decoder = protocol ?: return
        try {
            layoutAttempts += 1
            submit(
                RefloatPolling.layoutRequest(decoder, selectedRoute),
                VescExpectedResponse(VescCommand.CUSTOM_APP_DATA.id, RefloatCommand.REALTIME_DATA_INTERNAL_IDS.id),
                VescRequestPriority.CONTROLLER_WORK,
                VescRequestContext.RefloatLayout(selectedRoute.toRequestRoute()),
                REQUEST_TIMEOUT_MS,
            ) { outcome ->
                if (outcome == VescRequestCompletion.TimedOut) {
                    onDiagnostic("refloat_telemetry_layout_request_timeout")
                }
            }
        } catch (error: Exception) {
            onDiagnostic(error)
        }
    }

    private fun restart(intervalMs: Long) {
        val timer = scheduler ?: return
        pollingTimer?.cancel()
        pollingIntervalMs = intervalMs
        requestSample()
        pollingTimer = timer.scheduleRepeating(intervalMs, ::requestSample)
    }

    @Synchronized
    private fun requestSample() {
        if (!active) return
        if (deviceId == null) return
        val selectedRoute = route ?: return
        val decoder = protocol ?: return
        val nowMs = clock.monotonicMillis()
        if (pendingRequest != null) return
        try {
            val bytes: ByteArray
            val request: PendingRequest
            if (family == RefloatTelemetryFamily.LEGACY_FIXED) {
                request = PendingRequest.LEGACY
                bytes = RefloatPolling.telemetryRequest(decoder, selectedRoute, family)
            } else if (nextCompactDueMs?.let { nowMs >= it } == true) {
                request = PendingRequest.COMPACT
                nextCompactDueMs = nowMs + COMPACT_INTERVAL_MS
                bytes = RefloatPolling.compactRequest(decoder, selectedRoute)
            } else {
                request = PendingRequest.REALTIME
                bytes = RefloatPolling.telemetryRequest(decoder, selectedRoute, family)
            }
            pendingRequest = request
            pendingSinceMs = nowMs
            val command = when (request) {
                PendingRequest.REALTIME -> RefloatCommand.REALTIME_DATA_INTERNAL.id
                PendingRequest.COMPACT, PendingRequest.LEGACY -> RefloatCommand.GET_ALL_DATA.id
            }
            val ridingPriority = previousRideState == RideState.RIDING ||
                postRideProbeUntilMs?.let { nowMs <= it } == true
            submit(
                bytes,
                VescExpectedResponse(VescCommand.CUSTOM_APP_DATA.id, command),
                if (ridingPriority) VescRequestPriority.RIDING_TELEMETRY else VescRequestPriority.CONTROLLER_WORK,
                VescRequestContext.RefloatTelemetry(selectedRoute.toRequestRoute()),
                REQUEST_TIMEOUT_MS,
            ) { outcome ->
                if (outcome != VescRequestCompletion.TimedOut) return@submit
                synchronized(this) {
                    if (pendingRequest != request) return@synchronized
                    pendingRequest = null
                    pendingSinceMs = null
                    onDiagnostic("refloat_telemetry_request_timeout")
                }
            }
        } catch (error: Exception) {
            onDiagnostic(error)
        }
    }

    private fun responseMatchesPending(value: RefloatTelemetryRecord): Boolean {
        val matches = when {
            pendingRequest == PendingRequest.LEGACY &&
                family == RefloatTelemetryFamily.LEGACY_FIXED &&
                value.kind == RefloatTelemetryKind.COMPACT -> true
            pendingRequest == PendingRequest.REALTIME &&
                family == RefloatTelemetryFamily.DYNAMIC &&
                value.kind == RefloatTelemetryKind.REALTIME -> true
            pendingRequest == PendingRequest.COMPACT &&
                family == RefloatTelemetryFamily.DYNAMIC &&
                value.kind == RefloatTelemetryKind.COMPACT -> true
            else -> false
        }
        if (!matches) onDiagnostic("refloat_unexpected_telemetry_response")
        return matches
    }

    private fun normalizedSample(
        realtime: RefloatTelemetryRecord,
        sequence: Long,
        observedAt: Instant,
        elapsedMs: Long,
    ): TelemetrySample {
        val compact =
            if (family == RefloatTelemetryFamily.DYNAMIC) latestCompact else realtime to observedAt
        return TelemetrySample(
            sequence = sequence,
            observedAt = observedAt,
            elapsedMs = elapsedMs,
            rideState = realtime.rideState,
            slowObservedAt = compact?.second,
            packageState = realtime.packageState,
            packageMode = realtime.packageMode,
            footpadState = realtime.footpadState,
            stopCondition = realtime.stopCondition,
            setpointAdjustment = realtime.setpointAdjustment,
            wheelslip = realtime.wheelslip,
            darkride = realtime.darkride,
            charging = realtime.charging,
            alertReason = realtime.alertReason,
            firmwareFault = realtime.firmwareFault,
            packVoltageV = realtime.packVoltageV,
            dutyCycle = realtime.dutyCycle,
            controllerTempC = realtime.controllerTempC,
            motorTempC = realtime.motorTempC,
            motorCurrentA = realtime.motorCurrentA,
            batteryCurrentA = realtime.batteryCurrentA,
            erpm = realtime.erpm,
            speedKph = realtime.speedKph,
            pitchDeg = realtime.pitchDeg,
            balancePitchDeg = realtime.balancePitchDeg,
            rollDeg = realtime.rollDeg,
            directionalMotorCurrentA = realtime.directionalMotorCurrentA,
            filteredMotorCurrentA = realtime.filteredMotorCurrentA,
            requestedBalanceCurrentA = realtime.requestedBalanceCurrentA,
            boosterCurrentA = realtime.boosterCurrentA,
            fieldWeakeningCurrentA = compact?.first?.fieldWeakeningCurrentA,
            footpadAdc1V = realtime.footpadAdc1V,
            footpadAdc2V = realtime.footpadAdc2V,
            remoteInput = realtime.remoteInput,
            setpointDeg = realtime.setpointDeg,
            atrSetpointDeg = realtime.atrSetpointDeg,
            brakeTiltSetpointDeg = realtime.brakeTiltSetpointDeg,
            torqueTiltSetpointDeg = realtime.torqueTiltSetpointDeg,
            turnTiltSetpointDeg = realtime.turnTiltSetpointDeg,
            remoteSetpointDeg = realtime.remoteSetpointDeg,
            atrAccelerationDifferenceErpmPerSecond =
                realtime.atrAccelerationDifferenceErpmPerSecond,
            atrSpeedBoost = realtime.atrSpeedBoost,
            distanceM = compact?.first?.distanceM,
            odometerM = compact?.first?.odometerM,
            ampHoursDrawnAh = compact?.first?.ampHoursDrawnAh,
            ampHoursChargedAh = compact?.first?.ampHoursChargedAh,
            wattHoursDrawnWh = compact?.first?.wattHoursDrawnWh,
            wattHoursChargedWh = compact?.first?.wattHoursChargedWh,
            batteryLevel = compact?.first?.batteryLevel,
            chargingCurrentA = compact?.first?.chargingCurrentA,
            chargingVoltageV = compact?.first?.chargingVoltageV,
            extension = realtime.extensions,
        )
    }

    private companion object {
        const val RIDING_INTERVAL_MS = 100L
        const val COMPACT_INTERVAL_MS = 1_000L
        const val REQUEST_TIMEOUT_MS = 900L
        const val LAYOUT_RETRY_INTERVAL_MS = 1_000L
        const val MAX_LAYOUT_ATTEMPTS = 5
    }
}
