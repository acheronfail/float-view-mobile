package fail.acheron.floatview.core

class ApplicationService(
    private val bluetooth: BluetoothPort,
    private val clock: ClockPort,
    private val logStore: LogStorePort,
    private val protocol: VescProtocolPort? = null,
    private val scheduler: SchedulerPort? = null,
    samplingSettings: SamplingSettings = SamplingSettings(),
    private val events: EventSink,
) {
    private val sampling = AdaptiveSamplingPolicy(samplingSettings)
    private val reconnectDurationMs = samplingSettings.reconnectDurationMs
    private var explicitDisconnect = false
    private var scanSequence = 0
    private val knownDevicesById = linkedMapOf<DeviceId, DiscoveredDevice>()
    private var firmware: FirmwareRecord? = null
    private var transportFirmware: FirmwareRecord? = null
    private var refloatPackageVersion: String? = null
    private var pendingCanIds = mutableListOf<Int>()
    private var firmwareDiscoveryStarted = false
    private var telemetryLayoutTimer: CancellationPort? = null
    private var telemetryLayoutAttempts = 0
    private var telemetryTimer: CancellationPort? = null
    private var telemetryIntervalMs: Long? = null
    private var telemetryStartedAtMs: Long? = null
    private var telemetrySequence = 0L
    private var reconnectTimer: CancellationPort? = null
    private var reconnectStartedAtMs: Long? = null
    private var reconnectAttempt = 0
    var activeDeviceId: DeviceId? = null
        private set
    var connectionState: ConnectionState = ConnectionState.DISCONNECTED
        private set
    var bluetoothState: BluetoothState = bluetooth.currentState()
        private set
    var activeScanId: String? = null
        private set
    val knownDevices: List<DiscoveredDevice>
        get() = knownDevicesById.values.sortedWith(
            compareByDescending<DiscoveredDevice> { it.lastSeenAt }.thenBy { it.id.value },
        )

    @Synchronized
    fun requestBluetoothPermission(): BluetoothState {
        bluetooth.requestPermission()
        bluetoothState = bluetooth.currentState()
        return bluetoothState
    }

    @Synchronized
    fun startDiscovery(): String {
        check(bluetoothState.radio == BluetoothRadioState.POWERED_ON) {
            "Bluetooth is unavailable"
        }
        check(
            bluetoothState.scanPermission == BluetoothPermissionState.GRANTED ||
                bluetoothState.scanPermission == BluetoothPermissionState.NOT_REQUIRED,
        ) {
            "Bluetooth scan permission is required"
        }
        activeScanId?.let { return it }
        val scanId = "scan-${++scanSequence}"
        activeScanId = scanId
        bluetooth.startDiscovery()
        return scanId
    }

    @Synchronized
    fun stopDiscovery(scanId: String) {
        val active = activeScanId ?: return
        require(scanId == active) { "Unknown scan" }
        bluetooth.stopDiscovery()
        activeScanId = null
    }

    @Synchronized
    fun connect(deviceId: DeviceId) {
        require(knownDevicesById.containsKey(deviceId)) { "Unknown device" }
        require(activeDeviceId == null || activeDeviceId == deviceId) {
            "Only one board can be active"
        }
        if (activeScanId != null) {
            bluetooth.stopDiscovery()
            activeScanId = null
        }
        activeDeviceId = deviceId
        explicitDisconnect = false
        cancelReconnect()
        telemetryTimer?.cancel()
        telemetryTimer = null
        telemetryIntervalMs = null
        telemetryStartedAtMs = null
        telemetrySequence = 0
        resetFirmwareDiscovery()
        transition(ConnectionState.CONNECTING)
        bluetooth.connect(deviceId)
    }

    @Synchronized
    fun onBluetoothEvent(event: BluetoothEvent) {
        when (event) {
            is BluetoothEvent.StateChanged -> {
                bluetoothState = event.state
                if (event.state.radio != BluetoothRadioState.POWERED_ON && activeScanId != null) {
                    bluetooth.stopDiscovery()
                    activeScanId = null
                }
                if (
                    event.state.radio != BluetoothRadioState.POWERED_ON &&
                    activeDeviceId != null &&
                    logStore.active() != null &&
                    !explicitDisconnect
                ) {
                    beginReconnect()
                }
                events.publish(event.state)
            }
            is BluetoothEvent.DeviceFound -> {
                val device = DiscoveredDevice(
                    event.deviceId,
                    event.advertisedName,
                    event.rssi,
                    clock.now(),
                )
                knownDevicesById[event.deviceId] = device
                events.publish(device)
            }
            is BluetoothEvent.ScanFailed -> {
                activeScanId = null
                events.publish(event.reason)
            }
            is BluetoothEvent.ConnectionChanged -> {
                if (event.deviceId != activeDeviceId) return
                if (event.state == ConnectionState.READY && logStore.active() == null) {
                    try {
                        events.publish(logStore.create(event.deviceId, clock.now()))
                        telemetryStartedAtMs = clock.monotonicMillis()
                    } catch (error: Exception) {
                        transition(ConnectionState.FAILED)
                        explicitDisconnect = true
                        bluetooth.disconnect(event.deviceId)
                        events.publish(error)
                        return
                    }
                }
                if (event.state == ConnectionState.READY) {
                    cancelReconnect()
                    transition(ConnectionState.READY)
                    startFirmwareDiscovery(event.deviceId)
                    return
                }
                if (
                    (event.state == ConnectionState.FAILED ||
                        event.state == ConnectionState.DISCONNECTED) &&
                    !explicitDisconnect &&
                    logStore.active() != null
                ) {
                    beginReconnect()
                    return
                }
                transition(event.state)
                if (event.state == ConnectionState.DISCONNECTED) {
                    cancelReconnect()
                    activeDeviceId = null
                    resetFirmwareDiscovery()
                }
            }
            is BluetoothEvent.Notification -> {
                if (event.deviceId != activeDeviceId) return
                val decoder = protocol
                if (decoder == null) {
                    events.publish(event)
                    return
                }
                try {
                    decoder.receive(event.bytes).forEach { message ->
                        when (message) {
                            is VescProtocolMessage.Firmware -> handleFirmware(message.value)
                            is VescProtocolMessage.CanDevices -> {
                                pendingCanIds = message.canIds.toMutableList()
                                requestNextCanFirmware()
                            }
                            is VescProtocolMessage.RefloatInfo -> {
                                refloatPackageVersion = message.version
                                if (
                                    message.version == "1.2" ||
                                    message.version.startsWith("1.2.")
                                ) {
                                    requestRefloatTelemetryLayout()
                                }
                            }
                            is VescProtocolMessage.RefloatTelemetryLayout -> {
                                cancelTelemetryLayoutRetry()
                                startTelemetryPolling()
                            }
                            is VescProtocolMessage.RefloatTelemetry ->
                                handleRefloatTelemetry(message.value)
                        }
                        events.publish(message)
                    }
                } catch (error: Exception) {
                    events.publish(error)
                }
            }
        }
    }

    @Synchronized
    fun disconnect() {
        val deviceId = activeDeviceId ?: return
        transition(ConnectionState.DISCONNECTING)
        explicitDisconnect = true
        cancelReconnect()
        cancelTelemetryLayoutRetry()
        stopTelemetryPolling()
        logStore.active()?.let {
            events.publish(logStore.close(it.jobId, clock.now(), "explicit_disconnect"))
        }
        bluetooth.disconnect(deviceId)
    }

    @Synchronized
    fun ingest(sample: TelemetrySample) {
        val active = logStore.active() ?: return
        if (connectionState != ConnectionState.READY) return
        if (sampling.shouldPersist(sample.rideState, sample.elapsedMs)) {
            logStore.append(active.jobId, listOf(sample))
        }
        events.publish(sample)
    }

    @Synchronized
    fun discoverySnapshot() = DiscoverySnapshot(bluetoothState, activeScanId, knownDevices)

    @Synchronized
    fun activeConnectionSnapshot(): ActiveConnectionSnapshot? {
        val deviceId = activeDeviceId ?: return null
        return ActiveConnectionSnapshot(
            deviceId,
            knownDevicesById[deviceId]?.advertisedName,
            connectionState,
        )
    }

    @Synchronized
    fun firmwareSnapshot(): FirmwareRecord? = firmware

    @Synchronized
    fun refloatPackageVersionSnapshot(): String? = refloatPackageVersion

    @Synchronized
    fun requestFirmwareInfo(): FirmwareRecord? {
        val deviceId = activeDeviceId ?: return null
        check(connectionState == ConnectionState.READY) { "Device is not ready" }
        if (!firmwareDiscoveryStarted) startFirmwareDiscovery(deviceId)
        return firmware
    }

    private fun startFirmwareDiscovery(deviceId: DeviceId) {
        if (firmwareDiscoveryStarted) return
        val decoder = protocol ?: return
        firmwareDiscoveryStarted = true
        try {
            bluetooth.write(deviceId, decoder.firmwareRequest())
        } catch (error: Exception) {
            firmwareDiscoveryStarted = false
            events.publish(error)
        }
    }

    private fun handleFirmware(value: FirmwareRecord) {
        if (value.canId == null) {
            transportFirmware = value
            if (value.hardwareType == VESC_HARDWARE_TYPE) {
                firmware = value
                requestRefloatInfo(null)
            } else {
                val deviceId = activeDeviceId ?: return
                protocol?.let { bluetooth.write(deviceId, it.canPingRequest()) }
            }
            return
        }
        if (value.hardwareType == VESC_HARDWARE_TYPE && firmware == null) {
            firmware = value
            pendingCanIds.clear()
            requestRefloatInfo(value.canId)
            return
        }
        requestNextCanFirmware()
    }

    private fun requestRefloatInfo(canId: Int?) {
        val deviceId = activeDeviceId ?: return
        val decoder = protocol ?: return
        bluetooth.write(deviceId, decoder.refloatInfoRequest(canId))
    }

    private fun requestNextCanFirmware() {
        val deviceId = activeDeviceId ?: return
        val decoder = protocol ?: return
        val canId = pendingCanIds.removeFirstOrNull() ?: return
        bluetooth.write(deviceId, decoder.firmwareRequest(canId))
    }

    private fun requestRefloatTelemetryLayout() {
        if (telemetryLayoutTimer != null) return
        telemetryLayoutAttempts = 0
        sendRefloatTelemetryLayoutRequest()
        telemetryLayoutTimer = scheduler?.scheduleRepeating(
            TELEMETRY_LAYOUT_RETRY_INTERVAL_MS,
            ::retryRefloatTelemetryLayout,
        )
    }

    @Synchronized
    private fun retryRefloatTelemetryLayout() {
        if (telemetryLayoutTimer == null) return
        if (telemetryLayoutAttempts >= MAX_TELEMETRY_LAYOUT_ATTEMPTS) {
            cancelTelemetryLayoutRetry()
            failTelemetrySetup("refloat_telemetry_layout_timeout")
            return
        }
        sendRefloatTelemetryLayoutRequest()
    }

    private fun sendRefloatTelemetryLayoutRequest() {
        val deviceId = activeDeviceId ?: return
        val canId = firmware?.canId ?: return
        val decoder = protocol ?: return
        telemetryLayoutAttempts += 1
        bluetooth.write(deviceId, decoder.refloatTelemetryIdsRequest(canId))
    }

    private fun cancelTelemetryLayoutRetry() {
        telemetryLayoutTimer?.cancel()
        telemetryLayoutTimer = null
        telemetryLayoutAttempts = 0
    }

    private fun failTelemetrySetup(reason: String) {
        val deviceId = activeDeviceId ?: return
        transition(ConnectionState.FAILED)
        stopTelemetryPolling()
        logStore.active()?.let {
            events.publish(logStore.close(it.jobId, clock.now(), reason))
        }
        explicitDisconnect = true
        events.publish(reason)
        bluetooth.disconnect(deviceId)
    }

    private fun beginReconnect() {
        if (reconnectDurationMs <= 0) {
            finishReconnect("unexpected_disconnect")
            return
        }
        if (reconnectStartedAtMs == null) {
            reconnectStartedAtMs = clock.monotonicMillis()
            reconnectAttempt = 0
        }
        transition(ConnectionState.RECONNECTING)
        resetFirmwareDiscovery()
        scheduleReconnectAttempt()
    }

    private fun scheduleReconnectAttempt() {
        val timer = scheduler
        if (timer == null) {
            finishReconnect("reconnect_unavailable")
            return
        }
        if (reconnectTimer != null || reconnectStartedAtMs == null) return
        val elapsed = clock.monotonicMillis() - requireNotNull(reconnectStartedAtMs)
        val remaining = reconnectDurationMs - elapsed
        if (remaining <= 0) {
            finishReconnect("reconnect_timeout")
            return
        }
        val delay = minOf(1_000L shl minOf(reconnectAttempt, 5), MAX_RECONNECT_BACKOFF_MS, remaining)
        reconnectTimer = timer.schedule(delay, ::performReconnectAttempt)
    }

    @Synchronized
    private fun performReconnectAttempt() {
        reconnectTimer = null
        val deviceId = activeDeviceId ?: return
        if (connectionState != ConnectionState.RECONNECTING) return
        val elapsed = clock.monotonicMillis() - (reconnectStartedAtMs ?: return)
        if (elapsed >= reconnectDurationMs) {
            finishReconnect("reconnect_timeout")
            return
        }
        if (bluetoothState.radio != BluetoothRadioState.POWERED_ON) {
            scheduleReconnectAttempt()
            return
        }
        reconnectAttempt += 1
        bluetooth.connect(deviceId)
    }

    private fun finishReconnect(reason: String) {
        val deviceId = activeDeviceId
        cancelReconnect()
        logStore.active()?.let {
            events.publish(logStore.close(it.jobId, clock.now(), reason))
        }
        explicitDisconnect = true
        if (deviceId != null) bluetooth.disconnect(deviceId)
        activeDeviceId = null
        resetFirmwareDiscovery()
        transition(ConnectionState.DISCONNECTED)
        events.publish(reason)
    }

    private fun cancelReconnect() {
        reconnectTimer?.cancel()
        reconnectTimer = null
        reconnectStartedAtMs = null
        reconnectAttempt = 0
    }

    private fun startTelemetryPolling() {
        if (scheduler == null || firmware?.canId == null) return
        restartTelemetryPolling(IDLE_PROBE_INTERVAL_MS)
    }

    private fun restartTelemetryPolling(intervalMs: Long) {
        val timer = scheduler ?: return
        telemetryTimer?.cancel()
        telemetryIntervalMs = intervalMs
        requestTelemetrySample()
        telemetryTimer = timer.scheduleRepeating(intervalMs, ::requestTelemetrySample)
    }

    @Synchronized
    private fun requestTelemetrySample() {
        val deviceId = activeDeviceId ?: return
        val canId = firmware?.canId ?: return
        val decoder = protocol ?: return
        if (connectionState != ConnectionState.READY) return
        bluetooth.write(deviceId, decoder.refloatTelemetryRequest(canId))
    }

    private fun handleRefloatTelemetry(value: RefloatTelemetryRecord) {
        val nowMs = clock.monotonicMillis()
        if (telemetryStartedAtMs == null) telemetryStartedAtMs = nowMs
        val sample = TelemetrySample(
            sequence = ++telemetrySequence,
            observedAt = clock.now(),
            elapsedMs = maxOf(0, nowMs - requireNotNull(telemetryStartedAtMs)),
            rideState = value.rideState,
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
        ingest(sample)
        val target =
            if (value.rideState == RideState.RIDING) {
                RIDING_PROBE_INTERVAL_MS
            } else {
                IDLE_PROBE_INTERVAL_MS
            }
        if (target != telemetryIntervalMs) restartTelemetryPolling(target)
    }

    private fun stopTelemetryPolling() {
        telemetryTimer?.cancel()
        telemetryTimer = null
        telemetryIntervalMs = null
    }

    private fun transition(state: ConnectionState) {
        connectionState = state
        events.publish(state)
    }

    private fun resetFirmwareDiscovery() {
        cancelTelemetryLayoutRetry()
        stopTelemetryPolling()
        firmware = null
        transportFirmware = null
        refloatPackageVersion = null
        pendingCanIds.clear()
        firmwareDiscoveryStarted = false
        protocol?.reset()
    }

    private companion object {
        const val VESC_HARDWARE_TYPE = 0
        const val IDLE_PROBE_INTERVAL_MS = 500L
        const val RIDING_PROBE_INTERVAL_MS = 100L
        const val TELEMETRY_LAYOUT_RETRY_INTERVAL_MS = 1_000L
        const val MAX_TELEMETRY_LAYOUT_ATTEMPTS = 5
        const val MAX_RECONNECT_BACKOFF_MS = 30_000L
    }
}
