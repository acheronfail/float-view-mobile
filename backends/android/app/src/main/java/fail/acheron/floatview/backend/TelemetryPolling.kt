package fail.acheron.floatview.backend

import fail.acheron.floatview.protocol.refloat.RefloatPolling
import fail.acheron.floatview.protocol.refloat.RefloatTelemetryFamily
import fail.acheron.floatview.protocol.vesc.ControllerRoute

/**
 * Owns route-aware layout negotiation, polling cadence, and sample decoding.
 */
class TelemetryPolling(
    private val clock: ClockPort,
    private val scheduler: SchedulerPort?,
    private val protocol: VescProtocolPort?,
    private val write: (DeviceId, ByteArray) -> Unit,
    private val onSample: (TelemetrySample) -> Unit,
    private val onFailure: (String) -> Unit,
    private val onDiagnostic: (Any) -> Unit,
) {
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
    private var postRideTailMs = 30_000L
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

    fun receive(value: RefloatTelemetryRecord) {
        if (!active) return
        val nowMs = clock.monotonicMillis()
        if (startedAtMs == null) startedAtMs = nowMs
        val sample = TelemetrySample(
            sequence = ++sequence,
            observedAt = clock.now(),
            elapsedMs = maxOf(0, nowMs - requireNotNull(startedAtMs)),
            rideState = value.rideState,
            boardState = value.boardState,
            packVoltageV = value.packVoltageV,
            dutyCycle = value.dutyCycle,
            controllerTempC = value.controllerTempC,
            motorTempC = value.motorTempC,
            footpadState = value.footpadState,
            extension = mapOf(
                "motorCurrentA" to value.motorCurrentA,
                "batteryCurrentA" to value.batteryCurrentA,
                "erpm" to value.erpm.toDouble(),
                "speedKph" to value.speedKph,
            ),
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
        val target = deviceId ?: return
        val selectedRoute = route ?: return
        val decoder = protocol ?: return
        try {
            layoutAttempts += 1
            write(target, RefloatPolling.layoutRequest(decoder, selectedRoute))
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
        val target = deviceId ?: return
        val selectedRoute = route ?: return
        val decoder = protocol ?: return
        try {
            write(
                target,
                RefloatPolling.telemetryRequest(decoder, selectedRoute, family),
            )
        } catch (error: Exception) {
            onDiagnostic(error)
        }
    }

    private companion object {
        const val RIDING_INTERVAL_MS = 100L
        const val LAYOUT_RETRY_INTERVAL_MS = 1_000L
        const val MAX_LAYOUT_ATTEMPTS = 5
    }
}
