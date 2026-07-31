package fail.acheron.floatview.backend

import fail.acheron.floatview.protocol.refloat.RefloatTelemetryFamily
import fail.acheron.floatview.protocol.vesc.ControllerRoute

class ApplicationService(
    private val bluetooth: BluetoothPort,
    private val clock: ClockPort,
    private val logStore: LogStorePort,
    private val productStore: ProductStorePort? = null,
    private val sessionStore: SessionStorePort? = null,
    private val protocol: VescProtocolPort? = null,
    private val scheduler: SchedulerPort? = null,
    samplingSettings: SamplingSettings = SamplingSettings(),
    private val events: EventSink,
) {
    init {
        val savedIds = runCatching {
            productStore?.savedDevices().orEmpty()
                .map { DeviceId(it.bluetoothIdentifier) }
                .toSet()
        }.getOrDefault(emptySet())
        bluetooth.configureSavedDeviceIds(savedIds)
    }
    private var sampling = AdaptiveSamplingPolicy(samplingSettings)
    private var postRideTailMs = samplingSettings.postRideTailMs
    private var reconnectDurationMs = samplingSettings.reconnectDurationMs
    private var idleProbeIntervalMs = samplingSettings.idleProbeIntervalMs
    private var explicitDisconnect = false
    private var scanSequence = 0
    private val knownDevicesById = linkedMapOf<DeviceId, DiscoveredDevice>()
    private var discoveryExpiryTimer: CancellationPort? = null
    private var discoveryExpiryGeneration = 0L
    private val controllerDiscovery = ControllerDiscovery()
    private var currentSavedDevice: SavedDeviceRecord? = null
    private var identificationTimer: CancellationPort? = null
    private var canProbeTimer: CancellationPort? = null
    private var sessionControllerRoute: ControllerRoute? = null
    private var sessionControllerUuid: ByteArray? = null
    private var sessionFirmware: FirmwareRecord? = null
    private var sessionRefloatPackageVersion: String? = null
    private val telemetryPolling = TelemetryPolling(
        clock = clock,
        scheduler = scheduler,
        protocol = protocol,
        write = bluetooth::write,
        onSample = ::ingest,
        onFailure = { reason ->
            synchronized(this) {
                events.publish(reason)
                failIdentification(
                    ConnectionIssueCode.IDENTIFICATION_TIMEOUT,
                    "Refloat telemetry negotiation timed out.",
                )
            }
        },
        onDiagnostic = events::publish,
    )
    private var reconnectTimer: CancellationPort? = null
    private var reconnectStartedAtMs: Long? = null
    private var reconnectAttempt = 0
    private var latestPersistedSample: TelemetrySample? = null
    private var currentLimits: ControllerCurrentLimits? = null
    var activeDeviceId: DeviceId? = null
        private set
    private var activeDeviceName: String? = null
    var connectionState: ConnectionState = ConnectionState.DISCONNECTED
        private set
    var bluetoothState: BluetoothState = bluetooth.currentState()
        private set
    var activeScanId: String? = null
        private set
    val knownDevices: List<DiscoveredDevice>
        get() = knownDevicesById.values.sortedBy { it.id.value }

    @Synchronized
    fun requestBluetoothPermission(): BluetoothState {
        bluetooth.requestPermission()
        bluetoothState = bluetooth.currentState()
        return bluetoothState
    }

    fun savedDevices(): List<SavedDeviceRecord> =
        requireNotNull(productStore) { "Product store is unavailable" }.savedDevices()

    fun updateSavedDevice(
        savedDeviceId: String,
        expectedRevision: Int,
        customDisplayName: String?,
    ): SavedDeviceRecord =
        requireNotNull(productStore) { "Product store is unavailable" }
            .updateSavedDevice(savedDeviceId, expectedRevision, customDisplayName, clock.now())
            .also(events::publish)

    fun boardProfile(savedDeviceId: String): BoardProfileRecord =
        requireNotNull(productStore) { "Product store is unavailable" }
            .boardProfile(savedDeviceId)

    fun updateBoardProfile(
        savedDeviceId: String,
        expectedRevision: Int,
        patchJson: String,
    ): BoardProfileRecord =
        requireNotNull(productStore) { "Product store is unavailable" }
            .updateBoardProfile(savedDeviceId, expectedRevision, patchJson, clock.now())
            .also(events::publish)

    @Synchronized
    fun selectController(deviceId: DeviceId, canId: Int) {
        val store = productStore
        val savedDevice = currentSavedDevice
        check(
            activeDeviceId == deviceId &&
                connectionState == ConnectionState.SELECTING_CONTROLLER &&
                store != null &&
                savedDevice != null,
        ) { "Controller selection is not valid for the current state" }
        val effect = controllerDiscovery.selectController(canId)
        val selectedFirmware =
            (effect as ControllerDiscoveryEffect.RequestRefloatInfo).rememberController
                ?: error("Selected controller identity is unavailable")
        store.selectController(
            savedDevice.savedDeviceId,
            canId,
            ControllerDiscoveryPolicy.meaningfulUuid(selectedFirmware.controllerUuid),
            clock.now(),
        ).also(events::publish)
        controllerDiscovery.clearIssue()
        transition(ConnectionState.IDENTIFYING)
        performDiscoveryEffect(effect.copy(rememberController = null))
    }

    fun settings(): SettingsRecord =
        requireNotNull(productStore) { "Product store is unavailable" }.settings()

    @Synchronized
    fun updateSettings(expectedRevision: Int, patchJson: String): SettingsRecord =
        requireNotNull(productStore) { "Product store is unavailable" }
            .updateSettings(expectedRevision, patchJson)
            .also { value ->
                idleProbeIntervalMs = value.idleProbeIntervalMs
                postRideTailMs = value.postRideTailMs
                reconnectDurationMs = value.reconnectDurationMs
                sampling = AdaptiveSamplingPolicy(
                    SamplingSettings(
                        idleProbeIntervalMs = value.idleProbeIntervalMs,
                        idlePersistIntervalMs = value.idlePersistIntervalMs,
                        ridingPersistIntervalMs = value.ridingPersistIntervalMs,
                        postRideTailMs = value.postRideTailMs,
                        reconnectDurationMs = value.reconnectDurationMs,
                    ),
                )
                telemetryPolling.updateSettings(value.idleProbeIntervalMs, value.postRideTailMs)
                events.publish(value)
            }

    fun sessions(query: SessionListQuery): SessionPage =
        requireNotNull(sessionStore) { "Session store is unavailable" }.sessions(query)

    fun session(sessionId: String): SessionSummaryRecord =
        requireNotNull(sessionStore) { "Session store is unavailable" }.session(sessionId)

    fun querySessionSeries(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        fields: List<String>,
        maxPoints: Int,
    ): List<SessionSeriesPoint> =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .querySeries(sessionId, startElapsedMs, endElapsedMs, fields, maxPoints)

    fun sessionEvents(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
    ): List<SessionEventRecord> =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .sessionEvents(sessionId, startElapsedMs, endElapsedMs)

    fun sessionExtensionFields(sessionId: String): List<String> =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .sessionExtensionFields(sessionId)

    fun diagnosticWindow(
        sessionId: String,
        centerElapsedMs: Long,
        beforeMs: Long,
        afterMs: Long,
        fields: List<String>,
    ): List<SessionSeriesPoint> =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .diagnosticWindow(sessionId, centerElapsedMs, beforeMs, afterMs, fields)

    fun deleteSession(sessionId: String) =
        requireNotNull(sessionStore) { "Session store is unavailable" }.deleteSession(sessionId)

    fun exportSession(sessionId: String, format: String): SessionExportRecord =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .exportSession(sessionId, format)

    fun sessionGroups(): List<SessionGroupRecord> =
        requireNotNull(sessionStore) { "Session store is unavailable" }.sessionGroups()

    fun sessionGroup(sessionGroupId: String): SessionGroupRecord =
        requireNotNull(sessionStore) { "Session store is unavailable" }.sessionGroup(sessionGroupId)

    fun unlinkSessionGroup(sessionGroupId: String) =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .unlinkSessionGroup(sessionGroupId)

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
        events.publish(discoverySnapshot())
        return scanId
    }

    @Synchronized
    fun stopDiscovery(scanId: String) {
        val active = activeScanId ?: return
        require(scanId == active) { "Unknown scan" }
        bluetooth.stopDiscovery()
        activeScanId = null
        events.publish(discoverySnapshot())
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
        activeDeviceName = knownDevicesById[deviceId]?.advertisedName
        if (logStore.active() == null) {
            sessionControllerRoute = null
            sessionControllerUuid = null
            sessionFirmware = null
            sessionRefloatPackageVersion = null
        }
        latestPersistedSample = null
        explicitDisconnect = false
        cancelReconnect()
        telemetryPolling.resetSession()
        resetFirmwareDiscovery()
        transition(ConnectionState.CONNECTING)
        bluetooth.connect(deviceId)
    }

    @Synchronized
    fun onBluetoothEvent(event: BluetoothEvent) {
        when (event) {
            is BluetoothEvent.StateChanged -> {
                bluetoothState = event.state
                if (
                    (
                        event.state.radio != BluetoothRadioState.POWERED_ON ||
                            !permissionAllowsUse(event.state.scanPermission)
                        ) &&
                    activeScanId != null
                ) {
                    bluetooth.stopDiscovery()
                    activeScanId = null
                }
                if (
                    permissionIsTerminal(event.state.connectPermission) &&
                    activeDeviceId != null &&
                    !explicitDisconnect
                ) {
                    finishReconnect("bluetooth_permission_denied")
                } else if (
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
                scheduleDiscoveryExpiry()
                events.publish(device)
            }
            is BluetoothEvent.ScanFailed -> {
                activeScanId = null
                events.publish(event.reason)
            }
            is BluetoothEvent.ConnectionChanged -> {
                if (event.deviceId != activeDeviceId) return
                if (event.state == ConnectionState.READY) {
                    if (protocol == null) {
                        completeWithoutProtocol()
                        return
                    }
                    transition(ConnectionState.IDENTIFYING)
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
                if (
                    event.state == ConnectionState.FAILED &&
                    logStore.active() == null &&
                    !explicitDisconnect
                ) {
                    controllerDiscovery.markFailure(
                        ConnectionIssueCode.TRANSPORT_SETUP_FAILED,
                        "The VESC Bluetooth transport could not be prepared.",
                    )
                    transition(ConnectionState.FAILED)
                    return
                }
                if (
                    event.state == ConnectionState.DISCONNECTED &&
                    connectionState == ConnectionState.FAILED &&
                    !explicitDisconnect
                ) {
                    return
                }
                if (event.state == ConnectionState.DISCONNECTED) {
                    cancelReconnect()
                    activeDeviceId = null
                    activeDeviceName = null
                    latestPersistedSample = null
                    resetFirmwareDiscovery()
                }
                transition(event.state)
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
                            is VescProtocolMessage.CurrentLimits -> {
                                if (connectionState == ConnectionState.READY) {
                                    currentLimits = message.value
                                }
                            }
                            is VescProtocolMessage.CanDevices -> {
                                handleCanDevices(message.canIds)
                            }
                            is VescProtocolMessage.RefloatInfo -> {
                                if (connectionState != ConnectionState.IDENTIFYING) return@forEach
                                performDiscoveryEffect(
                                    controllerDiscovery.acceptRefloatInfo(message.version),
                                )
                            }
                            is VescProtocolMessage.RefloatTelemetryLayout -> {
                                telemetryPolling.layoutEstablished()
                                performDiscoveryEffect(
                                    controllerDiscovery.acceptTelemetryLayout(
                                        message.fields,
                                        message.runtimeFields,
                                    ),
                                )
                            }
                            is VescProtocolMessage.RefloatTelemetry ->
                                telemetryPolling.receive(message.value)
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
        telemetryPolling.cancelLayout()
        telemetryPolling.stop()
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
            latestPersistedSample = sample
            events.publish(sample)
        }
    }

    @Synchronized
    fun discoverySnapshot() = DiscoverySnapshot(bluetoothState, activeScanId, knownDevices)

    @Synchronized
    fun activeConnectionSnapshot(): ActiveConnectionSnapshot? {
        val deviceId = activeDeviceId ?: return null
        val savedDevice = productStore?.savedDevices()
            ?.firstOrNull { it.bluetoothIdentifier == deviceId.value }
        val activeSession = logStore.active()?.sessionId?.value?.let { sessionId ->
            runCatching { sessionStore?.session(sessionId) }.getOrNull()
        }
        return ActiveConnectionSnapshot(
            deviceId = deviceId,
            advertisedName = activeDeviceName,
            state = connectionState,
            savedDevice = currentSavedDevice ?: savedDevice,
            activeSession = activeSession,
            latestSample = latestPersistedSample,
            currentLimits = currentLimits,
            controllerRoute = controllerDiscovery.controllerRoute,
            controllerCandidates = controllerDiscovery.controllerCandidates,
            connectionIssue = controllerDiscovery.connectionIssue,
        )
    }

    @Synchronized
    fun firmwareSnapshot(): FirmwareRecord? = controllerDiscovery.firmware

    @Synchronized
    fun refloatPackageVersionSnapshot(): String? = controllerDiscovery.refloatPackageVersion

    private fun startFirmwareDiscovery(deviceId: DeviceId) {
        if (protocol == null || activeDeviceId != deviceId) return
        val (generation, effect) = controllerDiscovery.begin()
        if (effect == ControllerDiscoveryEffect.None) return
        identificationTimer?.cancel()
        identificationTimer = scheduler?.schedule(IDENTIFICATION_TIMEOUT_MS) {
            identificationTimedOut(generation)
        }
        performDiscoveryEffect(effect)
    }

    private fun handleFirmware(value: FirmwareRecord) {
        if (value.canId == null) {
            currentSavedDevice = try {
                val deviceId = activeDeviceId ?: return
                productStore?.ensureSavedDevice(
                    deviceId,
                    activeDeviceName,
                    clock.now(),
                )
            } catch (_: Exception) {
                failIdentification(
                    ConnectionIssueCode.TRANSPORT_SETUP_FAILED,
                    "Unable to save the VESC device.",
                )
                return
            }
            currentSavedDevice?.let { savedDevice ->
                val savedIds = runCatching {
                    productStore?.savedDevices().orEmpty()
                        .map { DeviceId(it.bluetoothIdentifier) }
                        .toSet()
                }.getOrDefault(setOf(DeviceId(savedDevice.bluetoothIdentifier)))
                bluetooth.configureSavedDeviceIds(savedIds)
            }
            val profile = currentSavedDevice?.let {
                runCatching { productStore?.boardProfile(it.savedDeviceId) }.getOrNull()
            }
            val hint = currentSavedDevice?.let {
                runCatching { productStore?.controllerRoutingHint(it.savedDeviceId) }.getOrNull()
            }
            performDiscoveryEffect(
                controllerDiscovery.acceptTransport(
                    value,
                    ControllerDiscoveryPolicy.preferredCanId(profile?.valuesJson),
                    hint,
                ),
            )
            return
        }
        if (controllerDiscovery.activeCanProbeId != value.canId) return
        cancelCanProbeTimeout()
        performDiscoveryEffect(
            controllerDiscovery.acceptCanFirmware(value, controllerContinuity()),
        )
    }

    private fun handleCanDevices(rawCanIds: List<Int>) {
        performDiscoveryEffect(
            controllerDiscovery.acceptCanIds(rawCanIds, controllerContinuity()),
        )
    }

    private fun controllerContinuity() = ControllerContinuity(
        hasActiveSession = logStore.active() != null,
        route = sessionControllerRoute,
        controllerUuid = sessionControllerUuid,
    )

    private fun performDiscoveryEffect(effect: ControllerDiscoveryEffect) {
        when (effect) {
            is ControllerDiscoveryEffect.ProbeFirmware -> {
                val deviceId = activeDeviceId ?: return
                val decoder = protocol ?: return
                val route = effect.route
                if (route is ControllerRoute.Can) scheduleCanProbeTimeout(route.canId)
                try {
                    bluetooth.write(deviceId, decoder.firmwareRequest(route))
                } catch (error: Exception) {
                    if (route == ControllerRoute.Direct) {
                        failIdentification(
                            ConnectionIssueCode.TRANSPORT_SETUP_FAILED,
                            "Unable to request VESC firmware.",
                        )
                        events.publish(error)
                    } else {
                        failIdentification(
                            ConnectionIssueCode.TRANSPORT_SETUP_FAILED,
                            "Unable to probe the CAN controller.",
                        )
                    }
                }
            }
            ControllerDiscoveryEffect.PingCan -> {
                cancelCanProbeTimeout()
                val deviceId = activeDeviceId ?: return
                val decoder = protocol ?: return
                try {
                    bluetooth.write(deviceId, decoder.canPingRequest())
                } catch (_: Exception) {
                    failIdentification(
                        ConnectionIssueCode.TRANSPORT_SETUP_FAILED,
                        "Unable to discover CAN controllers.",
                    )
                }
            }
            is ControllerDiscoveryEffect.ClearHintThen -> {
                invalidateAutomaticHint()
                restartIdentificationTimeout()
                performDiscoveryEffect(effect.next)
            }
            is ControllerDiscoveryEffect.RequestRefloatInfo -> {
                val selected = effect.rememberController
                val savedDevice = currentSavedDevice
                val canId = selected?.canId
                val uuid = ControllerDiscoveryPolicy.meaningfulUuid(selected?.controllerUuid)
                if (savedDevice != null && canId != null && uuid != null) {
                    runCatching {
                        productStore?.storeControllerRoutingHint(
                            ControllerRoutingHint(savedDevice.savedDeviceId, canId, uuid),
                            clock.now(),
                        )
                    }
                }
                val deviceId = activeDeviceId ?: return
                val decoder = protocol ?: return
                bluetooth.write(deviceId, decoder.refloatInfoRequest(effect.route))
            }
            ControllerDiscoveryEffect.RequestTelemetryLayout -> {
                val deviceId = activeDeviceId ?: return
                val route = controllerDiscovery.controllerRoute ?: return
                telemetryPolling.beginLayout(deviceId, route)
            }
            ControllerDiscoveryEffect.Ready -> completeApplicationReady()
            ControllerDiscoveryEffect.SelectingController -> {
                cancelIdentificationTimeout()
                transition(ConnectionState.SELECTING_CONTROLLER)
            }
            is ControllerDiscoveryEffect.Failed ->
                failIdentification(effect.issue.code, effect.issue.message)
            ControllerDiscoveryEffect.None -> Unit
        }
    }

    private fun completeApplicationReady() {
        val deviceId = activeDeviceId ?: return
        val route = controllerDiscovery.controllerRoute ?: return
        val selectedFirmware = controllerDiscovery.firmware ?: return
        cancelIdentificationTimeout()
        if (logStore.active() != null) {
            if (!controllerContinuityIsProven(route, selectedFirmware)) {
                failIdentification(
                    ConnectionIssueCode.CONTROLLER_CHANGED,
                    "The connected controller no longer matches the active session.",
                )
                return
            }
        } else {
            try {
                events.publish(
                    logStore.create(
                        deviceId,
                        activeDeviceName,
                        clock.now(),
                    ),
                )
                telemetryPolling.markSessionStarted()
                sessionControllerRoute = route
                sessionControllerUuid = ControllerDiscoveryPolicy.meaningfulUuid(
                    selectedFirmware.controllerUuid,
                )
                sessionFirmware = selectedFirmware
                sessionRefloatPackageVersion = controllerDiscovery.refloatPackageVersion
            } catch (_: Exception) {
                failIdentification(
                    ConnectionIssueCode.TRANSPORT_SETUP_FAILED,
                    "Unable to start durable telemetry.",
                )
                return
            }
        }
        cancelReconnect()
        controllerDiscovery.clearIssue()
        clearDiscoveredDevices(publish = false)
        transition(ConnectionState.READY)
        protocol?.let { decoder ->
            runCatching {
                bluetooth.write(deviceId, decoder.motorConfigurationRequest(route))
            }.onFailure(events::publish)
        }
        telemetryPolling.start(
            deviceId,
            route,
            controllerDiscovery.refloatFamily ?: RefloatTelemetryFamily.DYNAMIC,
            idleProbeIntervalMs,
            postRideTailMs,
        )
    }

    /** Production hosts always provide a VESC codec and never take this path. */
    private fun completeWithoutProtocol() {
        val deviceId = activeDeviceId ?: return
        if (logStore.active() == null) {
            try {
                events.publish(
                    logStore.create(
                        deviceId,
                        activeDeviceName,
                        clock.now(),
                    ),
                )
                telemetryPolling.markSessionStarted()
            } catch (error: Exception) {
                transition(ConnectionState.FAILED)
                explicitDisconnect = true
                bluetooth.disconnect(deviceId)
                events.publish(error)
                return
            }
        }
        cancelReconnect()
        clearDiscoveredDevices(publish = false)
        transition(ConnectionState.READY)
    }

    private fun controllerContinuityIsProven(
        route: ControllerRoute,
        selectedFirmware: FirmwareRecord,
    ): Boolean {
        val previousRoute = sessionControllerRoute ?: return true
        val previousUuid = sessionControllerUuid
        val currentUuid = ControllerDiscoveryPolicy.meaningfulUuid(selectedFirmware.controllerUuid)
        if (previousUuid != null) return currentUuid?.contentEquals(previousUuid) == true
        return previousRoute == route &&
            sessionFirmware?.let {
                ControllerDiscoveryPolicy.samePublicIdentity(it, selectedFirmware)
            } == true &&
            sessionRefloatPackageVersion == controllerDiscovery.refloatPackageVersion
    }

    @Synchronized
    private fun identificationTimedOut(generation: Long) {
        if (connectionState != ConnectionState.IDENTIFYING) return
        performDiscoveryEffect(controllerDiscovery.identificationTimedOut(generation))
    }

    private fun failIdentification(code: ConnectionIssueCode, message: String) {
        cancelIdentificationTimeout()
        cancelCanProbeTimeout()
        telemetryPolling.cancelLayout()
        telemetryPolling.stop()
        controllerDiscovery.markFailure(code, message)
        if (code == ConnectionIssueCode.CONTROLLER_CHANGED) {
            logStore.active()?.let {
                runCatching { logStore.close(it.jobId, clock.now(), code.name.lowercase()) }
            }
        }
        transition(ConnectionState.FAILED)
    }

    private fun cancelIdentificationTimeout() {
        identificationTimer?.cancel()
        identificationTimer = null
    }

    private fun restartIdentificationTimeout() {
        identificationTimer?.cancel()
        val generation = controllerDiscovery.identificationGeneration
        identificationTimer = scheduler?.schedule(IDENTIFICATION_TIMEOUT_MS) {
            identificationTimedOut(generation)
        }
    }

    private fun invalidateAutomaticHint() {
        currentSavedDevice?.savedDeviceId?.let { savedDeviceId ->
            runCatching { productStore?.clearControllerRoutingHint(savedDeviceId) }
        }
    }

    private fun scheduleCanProbeTimeout(canId: Int) {
        cancelCanProbeTimeout()
        val generation = controllerDiscovery.canProbeGeneration
        canProbeTimer = scheduler?.schedule(CAN_PROBE_TIMEOUT_MS) {
            canProbeTimedOut(canId, generation)
        }
    }

    @Synchronized
    private fun canProbeTimedOut(canId: Int, generation: Long) {
        if (connectionState != ConnectionState.IDENTIFYING) return
        val effect = controllerDiscovery.canProbeTimedOut(canId, generation)
        if (effect == ControllerDiscoveryEffect.None) return
        cancelCanProbeTimeout()
        performDiscoveryEffect(effect)
    }

    private fun cancelCanProbeTimeout() {
        canProbeTimer?.cancel()
        canProbeTimer = null
    }

    private fun scheduleDiscoveryExpiry() {
        if (activeScanId == null) return
        val timer = scheduler ?: return
        discoveryExpiryTimer?.cancel()
        val generation = ++discoveryExpiryGeneration
        discoveryExpiryTimer = timer.schedule(DISCOVERY_EXPIRY_MS) {
            synchronized(this) {
                if (discoveryExpiryGeneration != generation) return@synchronized
                discoveryExpiryTimer = null
                clearDiscoveredDevices(publish = true)
            }
        }
    }

    private fun clearDiscoveredDevices(publish: Boolean) {
        discoveryExpiryGeneration++
        discoveryExpiryTimer?.cancel()
        discoveryExpiryTimer = null
        if (knownDevicesById.isEmpty()) return
        knownDevicesById.clear()
        if (publish) events.publish(discoverySnapshot())
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
        activeDeviceName = null
        latestPersistedSample = null
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

    private fun transition(state: ConnectionState) {
        connectionState = state
        events.publish(state)
    }

    private fun resetFirmwareDiscovery() {
        telemetryPolling.cancelLayout()
        telemetryPolling.stop()
        controllerDiscovery.reset()
        currentLimits = null
        currentSavedDevice = null
        cancelIdentificationTimeout()
        cancelCanProbeTimeout()
        protocol?.reset()
    }

    private fun permissionAllowsUse(state: BluetoothPermissionState): Boolean =
        state == BluetoothPermissionState.GRANTED ||
            state == BluetoothPermissionState.NOT_REQUIRED

    private fun permissionIsTerminal(state: BluetoothPermissionState): Boolean =
        state == BluetoothPermissionState.DENIED ||
            state == BluetoothPermissionState.RESTRICTED

    private companion object {
        const val MAX_RECONNECT_BACKOFF_MS = 30_000L
        const val DISCOVERY_EXPIRY_MS = 120_000L
        const val IDENTIFICATION_TIMEOUT_MS = 10_000L
        const val CAN_PROBE_TIMEOUT_MS = 1_000L
    }
}
