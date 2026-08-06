package fail.acheron.floatview.backend

import fail.acheron.floatview.protocol.refloat.RefloatTelemetryFamily
import fail.acheron.floatview.protocol.refloat.RefloatCommand
import fail.acheron.floatview.protocol.refloat.RefloatBooleanControl
import fail.acheron.floatview.protocol.refloat.RefloatCompatibility
import fail.acheron.floatview.protocol.refloat.RefloatControlCodec
import fail.acheron.floatview.protocol.refloat.RefloatControlProfile
import fail.acheron.floatview.protocol.refloat.RefloatLightingState
import fail.acheron.floatview.protocol.vesc.ControllerRoute
import fail.acheron.floatview.protocol.vesc.VescCommandRouter
import fail.acheron.floatview.protocol.vesc.VescCompatibility
import fail.acheron.floatview.protocol.VescFrameCodec
import fail.acheron.floatview.protocol.vesc.VescCommand
import java.security.MessageDigest
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

class ApplicationService(
    private val bluetooth: BluetoothPort,
    private val clock: ClockPort,
    private val logStore: LogStorePort,
    private val productStore: ProductStorePort? = null,
    private val sessionStore: SessionStorePort? = null,
    private val protocol: VescProtocolPort? = null,
    private val scheduler: SchedulerPort? = null,
    samplingSettings: SamplingSettings = SamplingSettings(),
    private val locationProvider: LocationProviderPort? = null,
    private val events: EventSink,
) {
    init {
        val savedIds = runCatching {
            productStore?.savedDevices().orEmpty()
                .mapNotNull { it.bluetoothIdentifier?.let(::DeviceId) }
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
    private val auxiliaryDiscovery = AuxiliaryNodeDiscovery()
    private var trackingMode = TrackingMode.SAVED
    private var currentSavedDevice: SavedDeviceRecord? = null
    private var identificationTimer: CancellationPort? = null
    private var canProbeTimer: CancellationPort? = null
    private var bmsPollTimer: CancellationPort? = null
    private var bmsFailureCount = 0
    private var sessionControllerRoute: ControllerRoute? = null
    private var sessionControllerUuid: ByteArray? = null
    private var sessionFirmware: FirmwareRecord? = null
    private var sessionRefloatPackageVersion: String? = null
    private var latestLocation: LiveLocation? = null
    private var vescGeneration = VescRequestGeneration(0)
    private val vescRequests: VescRequestScheduler by lazy {
        VescRequestScheduler(
            clock,
            scheduler ?: SystemScheduler(),
            write = { bytes -> activeDeviceId?.let { bluetooth.write(it, bytes) } },
            diagnostic = events::publish,
        )
    }
    private val telemetryPolling = TelemetryPolling(
        clock = clock,
        scheduler = scheduler,
        protocol = protocol,
        submit = ::submitVescRequest,
        onSample = ::ingest,
        onFailure = { reason ->
            synchronized(this) {
                events.publish(reason)
                if (refloatMutations.controllerActions(false).pendingAction == ControllerPendingAction.SHUTDOWN) {
                    shutdownIntentTimer?.cancel()
                    shutdownIntentTimer = null
                    refloatMutations.clearShutdown(vescGeneration)
                    finishReconnect("controller_shutdown")
                    return@synchronized
                }
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
    private var locationCaptureStartedMonotonicMs: Long? = null
    private var locationCaptureBaseElapsedMs = 0L
    private var locationCaptureActive = false
    private var currentLimits: ControllerCurrentLimits? = null
    private val refloatMutations = DeviceMutationCoordinator()
    private var refloatProfile: RefloatControlProfile? = null
    private var latestRefloatTelemetry: RefloatTelemetryRecord? = null
    private var refloatRefreshTimer: CancellationPort? = null
    private var refloatMutationTimer: CancellationPort? = null
    private var shutdownIntentTimer: CancellationPort? = null
    private var refloatWaiter: RefloatMutationWaiter? = null
    private val refloatScheduler: SchedulerPort by lazy { scheduler ?: SystemScheduler() }
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

    @Synchronized
    fun deleteSavedDevice(savedDeviceId: String) {
        if (activeConnectionSnapshot()?.savedDevice?.savedDeviceId == savedDeviceId) {
            throw ProductStoreException.SavedDeviceActive
        }
        requireNotNull(productStore) { "Product store is unavailable" }.deleteSavedDevice(savedDeviceId)
    }

    fun boardProfile(savedDeviceId: String): BoardProfileRecord =
        requireNotNull(productStore) { "Product store is unavailable" }
            .boardProfile(savedDeviceId)

    @Synchronized
    fun updateBoardProfile(
        savedDeviceId: String,
        expectedRevision: Int,
        patchJson: String,
    ): BoardProfileRecord =
        requireNotNull(productStore) { "Product store is unavailable" }
            .updateBoardProfile(savedDeviceId, expectedRevision, patchJson, clock.now())
            .also { profile ->
                events.publish(profile)
                if (
                    connectionState == ConnectionState.READY &&
                    currentSavedDevice?.savedDeviceId == savedDeviceId
                ) {
                    bmsPollTimer?.cancel()
                    bmsPollTimer = null
                    startAuxiliaryDiscovery()
                }
            }

    @Synchronized
    fun selectController(deviceId: DeviceId, canId: Int) {
        check(
            activeDeviceId == deviceId &&
                connectionState == ConnectionState.SELECTING_CONTROLLER,
        ) { "Controller selection is not valid for the current state" }
        val effect = controllerDiscovery.selectController(canId)
        val selectedFirmware =
            (effect as ControllerDiscoveryEffect.RequestRefloatInfo).rememberController
                ?: error("Selected controller identity is unavailable")
        if (trackingMode == TrackingMode.SAVED) {
            val store = requireNotNull(productStore) { "Product store is unavailable" }
            val savedDevice = requireNotNull(currentSavedDevice) { "Saved board is unavailable" }
            store.selectController(
                savedDevice.savedDeviceId,
                canId,
                ControllerDiscoveryPolicy.meaningfulUuid(selectedFirmware.controllerUuid),
                clock.now(),
            ).also(events::publish)
        }
        controllerDiscovery.clearIssue()
        transition(ConnectionState.IDENTIFYING)
        performDiscoveryEffect(effect.copy(rememberController = null))
    }

    fun settings(): SettingsRecord =
        requireNotNull(productStore) { "Product store is unavailable" }.settings()

    @Synchronized
    fun locationState(): LocationProviderState {
        val enabled = runCatching { settings().locationRecordingEnabled }.getOrDefault(false)
        if (!enabled) {
            return LocationProviderState(
                LocationProviderAuthorization.UNAVAILABLE,
                LocationProviderBackgroundCapture.UNAVAILABLE,
            )
        }
        return locationProvider?.state() ?: LocationProviderState(
            LocationProviderAuthorization.UNAVAILABLE,
            LocationProviderBackgroundCapture.UNAVAILABLE,
            "This host does not provide location capture.",
        )
    }

    @Synchronized
    fun requestLocationPermission(): LocationProviderState {
        locationProvider?.requestPermission()
        return locationState()
    }

    @Synchronized
    fun openLocationSettings(): LocationProviderState {
        locationProvider?.openSettings()
        return locationState()
    }

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
                if (!value.locationRecordingEnabled) stopLocationCapture()
                else locationProvider?.requestPermission()
                events.publish(value)
            }

    fun sessions(query: SessionListQuery): SessionPage =
        requireNotNull(sessionStore) { "Session store is unavailable" }.sessions(query)

    fun session(sessionId: String): SessionSummaryRecord =
        requireNotNull(sessionStore) { "Session store is unavailable" }.session(sessionId)

    fun sessionSnapshot(sessionId: String): SessionPublishedSnapshotRecord =
        requireNotNull(sessionStore) { "Session store is unavailable" }.sessionSnapshot(sessionId)

    fun querySessionSeries(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        fields: List<String>,
        maxPoints: Int,
        publishedSampleSequence: Long? = null,
    ): List<SessionSeriesPoint> =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .querySeries(sessionId, startElapsedMs, endElapsedMs, fields, maxPoints, publishedSampleSequence)

    fun querySessionSeriesSource(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        fields: List<String>,
        maxRows: Int,
        publishedSampleSequence: Long? = null,
    ): List<SessionSeriesPoint> =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .querySeriesSource(sessionId, startElapsedMs, endElapsedMs, fields, maxRows, publishedSampleSequence)

    fun reduceSessionSeries(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        fields: List<String>,
        buckets: Int,
        gapPolicy: SessionGapPolicyRecord,
        expectedPersistenceMs: Long,
        expectedRidingPersistenceMs: Long?,
        expectedIdlePersistenceMs: Long?,
        retainedResolutionSeconds: Long,
        publishedSampleSequence: Long? = null,
    ): SessionSeriesReductionRecord =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .reduceSeries(
                sessionId = sessionId,
                startElapsedMs = startElapsedMs,
                endElapsedMs = endElapsedMs,
                fields = fields,
                buckets = buckets,
                gapPolicy = gapPolicy,
                expectedPersistenceMs = expectedPersistenceMs,
                expectedRidingPersistenceMs = expectedRidingPersistenceMs,
                expectedIdlePersistenceMs = expectedIdlePersistenceMs,
                retainedResolutionSeconds = retainedResolutionSeconds,
                publishedSampleSequence = publishedSampleSequence,
            )

    fun sessionEvents(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        publishedEventSequence: Long? = null,
    ): List<SessionEventRecord> =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .sessionEvents(sessionId, startElapsedMs, endElapsedMs, publishedEventSequence)

    fun querySessionLocations(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        limit: Int,
        cursor: String?,
        publishedLocationSequence: Long?,
    ): SessionLocationPageRecord =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .queryLocations(sessionId, startElapsedMs, endElapsedMs, limit, cursor, publishedLocationSequence)

    fun sessionExtensionFields(sessionId: String): List<String> =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .sessionExtensionFields(sessionId)

    fun sessionHistoryConfiguration(sessionId: String): SessionHistoryConfiguration =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .sessionHistoryConfiguration(sessionId)

    fun sessionPruneMetadata(sessionId: String): SessionPruneMetadataRecord =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .sessionPruneMetadata(sessionId)

    fun <T> withSessionReadCancellation(cancellation: SessionReadCancellation, body: () -> T): T =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .withSessionReadCancellation(cancellation, body)

    fun cancelHistoricalReads(sessionId: String) {
        sessionStore?.cancelHistoricalReads(sessionId)
    }

    fun prunePreview(sessionId: String, targetResolutionSeconds: Long): SessionPrunePreviewRecord =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .prunePreview(sessionId, targetResolutionSeconds)

    fun pruneSession(sessionId: String, targetResolutionSeconds: Long): SessionPrunePreviewRecord =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .pruneSession(sessionId, targetResolutionSeconds)

    fun maintenanceJob(): SessionMaintenanceJobRecord? = sessionStore?.maintenanceJob()

    fun saveMaintenanceJob(job: SessionMaintenanceJobRecord) {
        requireNotNull(sessionStore) { "Session store is unavailable" }.saveMaintenanceJob(job)
    }

    fun cancelMaintenanceJob(operationId: String): SessionMaintenanceJobRecord? =
        requireNotNull(sessionStore) { "Session store is unavailable" }.cancelMaintenanceJob(operationId)

    fun querySampleWindow(
        sessionId: String,
        centerElapsedMs: Long,
        beforeMs: Long,
        afterMs: Long,
        fields: List<String>,
        publishedSampleSequence: Long? = null,
    ): List<SessionSeriesPoint> =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .querySampleWindow(sessionId, centerElapsedMs, beforeMs, afterMs, fields, publishedSampleSequence)

    fun deleteSession(sessionId: String) =
        requireNotNull(sessionStore) { "Session store is unavailable" }.deleteSession(sessionId)

    fun cancelPendingDelete(operationId: String): Boolean =
        requireNotNull(sessionStore) { "Session store is unavailable" }.cancelPendingDelete(operationId)

    fun exportSession(sessionId: String, format: String): SessionExportRecord =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .exportSession(sessionId, format)

    fun importSessionArchive(data: ByteArray): String =
        requireNotNull(sessionStore) { "Session store is unavailable" }
            .importSessionArchive(data)

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
    fun connect(deviceId: DeviceId, trackingMode: TrackingMode = TrackingMode.SAVED) {
        require(knownDevicesById.containsKey(deviceId)) { "Unknown device" }
        require(activeDeviceId == null || activeDeviceId == deviceId) {
            "Only one board can be active"
        }
        if (activeDeviceId == deviceId && connectionState == ConnectionState.FAILED && this.trackingMode != trackingMode) {
            error("A retry must keep the active tracking mode")
        }
        if (activeScanId != null) {
            bluetooth.stopDiscovery()
            activeScanId = null
        }
        activeDeviceId = deviceId
        this.trackingMode = trackingMode
        currentSavedDevice = null
        activeDeviceName = knownDevicesById[deviceId]?.advertisedName
        if (logStore.active() == null) {
            sessionControllerRoute = null
            sessionControllerUuid = null
            sessionFirmware = null
            sessionRefloatPackageVersion = null
        }
        latestPersistedSample = null
        latestLocation = null
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
                    (trackingMode == TrackingMode.SAVED || trackingMode == TrackingMode.GUEST) &&
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
                    savedDeviceId = savedDeviceIdFor(event.deviceId),
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
                if (
                    (event.state == ConnectionState.FAILED || event.state == ConnectionState.DISCONNECTED) &&
                    refloatMutations.controllerActions(false).pendingAction == ControllerPendingAction.SHUTDOWN
                ) {
                    shutdownIntentTimer?.cancel()
                    shutdownIntentTimer = null
                    refloatMutations.clearShutdown(vescGeneration)
                    finishReconnect("controller_shutdown")
                    return
                }
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
                    (trackingMode == TrackingMode.SAVED || trackingMode == TrackingMode.GUEST)
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
                    latestLocation = null
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
                        val requestContext = vescRequests.inFlightRequest()?.context
                        val identity = when (message) {
                            is VescProtocolMessage.Firmware -> VescCommand.GET_FIRMWARE_VERSION.id to null
                            is VescProtocolMessage.CurrentLimits -> VescCommand.GET_MOTOR_CONFIGURATION.id to null
                            is VescProtocolMessage.CanDevices -> VescCommand.PING_CAN.id to null
                            is VescProtocolMessage.BmsValuesPayload -> VescCommand.BMS_GET_VALUES.id to null
                            is VescProtocolMessage.RefloatInfo -> VescCommand.CUSTOM_APP_DATA.id to RefloatCommand.INFO.id
                            is VescProtocolMessage.RefloatTelemetryLayout ->
                                VescCommand.CUSTOM_APP_DATA.id to RefloatCommand.REALTIME_DATA_INTERNAL_IDS.id
                            is VescProtocolMessage.RefloatTelemetry -> VescCommand.CUSTOM_APP_DATA.id to
                                if (message.value.kind == RefloatTelemetryKind.REALTIME) {
                                    RefloatCommand.REALTIME_DATA_INTERNAL.id
                                } else {
                                    RefloatCommand.GET_ALL_DATA.id
                                }
                            is VescProtocolMessage.RefloatLighting ->
                                VescCommand.CUSTOM_APP_DATA.id to message.command.id
                        }
                        if (
                            !vescRequests.receive(event.bytes, identity.first, identity.second)
                        ) return@forEach
                        when (message) {
                            is VescProtocolMessage.Firmware -> {
                                val attributed = attributedFirmware(message.value, requestContext)
                                if (requestContext is VescRequestContext.AuxiliaryFirmware) {
                                    events.publish(
                                        "bms_discovery_identity:${bmsRouteDiagnostic(attributed.canId)}:" +
                                            "type=${attributed.hardwareType}:" +
                                            "version=${attributed.major}.${attributed.minor}:" +
                                            "hardware=${bmsHardwareDiagnostic(attributed.hardwareName)}",
                                    )
                                    performAuxiliaryEffect(
                                        auxiliaryDiscovery.acceptFirmware(
                                            attributed, auxiliaryDiscovery.generation,
                                        ),
                                    )
                                } else {
                                    handleFirmware(attributed)
                                }
                            }
                            is VescProtocolMessage.CurrentLimits -> {
                                if (connectionState == ConnectionState.READY) {
                                    currentLimits = message.value
                                }
                            }
                            is VescProtocolMessage.CanDevices -> {
                                if (requestContext == VescRequestContext.AuxiliaryCanPing) {
                                    events.publish("bms_discovery_scan_result:count=${message.canIds.size}")
                                    performAuxiliaryEffect(
                                        auxiliaryDiscovery.acceptCanIds(
                                            message.canIds, auxiliaryDiscovery.generation,
                                        ),
                                    )
                                } else {
                                    handleCanDevices(message.canIds)
                                }
                            }
                            is VescProtocolMessage.BmsValuesPayload -> {
                                val route = (requestContext as? VescRequestContext.BmsValues)?.route
                                    ?: return@forEach
                                val expectedCanId = (route as? VescRequestRoute.Can)?.canId
                                try {
                                    val snapshot = fail.acheron.floatview.protocol.VescPayloadDecoder.bmsValues(
                                        message.payload, expectedCanId, clock.now(),
                                    )
                                    bmsFailureCount = 0
                                    auxiliaryDiscovery.acceptSnapshot(snapshot)
                                    events.publish("bms_values_attached:${bmsRouteDiagnostic(expectedCanId)}")
                                    events.publish(snapshot)
                                } catch (_: Exception) {
                                    events.publish("bms_values_decode_failed:${bmsRouteDiagnostic(expectedCanId)}")
                                    recordBmsFailure()
                                }
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
                            is VescProtocolMessage.RefloatTelemetry -> {
                                telemetryPolling.receive(message.value)
                                latestRefloatTelemetry = message.value
                                val pending = refloatMutations.snapshot()?.pendingAction
                                val updated = refloatMutations.observeTelemetry(message.value)
                                if (pending != null && updated?.pendingAction == null) finishRefloatMutation(Result.success(updated!!))
                            }
                            is VescProtocolMessage.RefloatLighting -> {
                                val pending = refloatMutations.snapshot()?.pendingAction
                                val updated = refloatMutations.observeLighting(message.state)
                                if (pending != null && updated?.pendingAction == null) finishRefloatMutation(Result.success(updated!!))
                            }
                        }
                        if (message !is VescProtocolMessage.BmsValuesPayload) events.publish(message)
                    }
                } catch (error: Exception) {
                    events.publish(error)
                }
            }
        }
    }

    private fun attributedFirmware(value: FirmwareRecord, context: VescRequestContext?): FirmwareRecord {
        val canId = when (context) {
            is VescRequestContext.ControllerFirmware -> (context.route as? VescRequestRoute.Can)?.canId
            is VescRequestContext.AuxiliaryFirmware -> (context.route as? VescRequestRoute.Can)?.canId
            else -> null
        }
        return value.copy(canId = canId ?: value.canId)
    }

    @Synchronized
    fun disconnect() {
        val deviceId = activeDeviceId ?: return
        transition(ConnectionState.DISCONNECTING)
        explicitDisconnect = true
        cancelReconnect()
        telemetryPolling.cancelLayout()
        telemetryPolling.stop()
        stopLocationCapture()
        if (trackingMode == TrackingMode.SAVED) {
            logStore.active()?.let {
                events.publish(logStore.close(it.jobId, clock.now(), "explicit_disconnect"))
            }
        }
        bluetooth.disconnect(deviceId)
    }

    @Synchronized
    fun ingest(sample: TelemetrySample) {
        if (connectionState != ConnectionState.READY) return
        if (trackingMode == TrackingMode.GUEST) {
            latestPersistedSample = sample
            events.publish(sample)
            return
        }
        val active = logStore.active() ?: return
        updateLocationCapture(sample.rideState)
        if (sampling.shouldPersist(sample.rideState, sample.elapsedMs)) {
            logStore.append(active.jobId, listOf(sample))
            latestPersistedSample = sample
            events.publish(sample)
        }
    }

    @Synchronized
    private fun updateLocationCapture(rideState: RideState) {
        val provider = locationProvider
        val active = logStore.active()
        val enabled = trackingMode == TrackingMode.SAVED &&
            runCatching { settings().locationRecordingEnabled }.getOrDefault(false)
        val providerState = provider?.state()
        val eligible = enabled && rideState == RideState.RIDING && active != null &&
            (providerState?.authorization == LocationProviderAuthorization.AUTHORIZED ||
                providerState?.authorization == LocationProviderAuthorization.FOREGROUND_ONLY)
        if (!eligible) {
            stopLocationCapture()
            return
        }
        if (locationCaptureActive) return
        val activeSession = requireNotNull(active)
        val activeProvider = requireNotNull(provider)
        locationCaptureActive = true
        locationCaptureStartedMonotonicMs = clock.monotonicMillis()
        locationCaptureBaseElapsedMs = latestPersistedSample?.elapsedMs ?: 0L
        val jobId = activeSession.jobId
        activeProvider.startUpdates { observation ->
            ingestLocation(observation, jobId)
        }
    }

    @Synchronized
    private fun stopLocationCapture() {
        if (!locationCaptureActive) return
        locationCaptureActive = false
        locationCaptureStartedMonotonicMs = null
        locationProvider?.stopUpdates()
    }

    @Synchronized
    private fun ingestLocation(observation: LocationProviderObservation, jobId: JobId) {
        val started = locationCaptureStartedMonotonicMs ?: return
        if (!locationCaptureActive || logStore.active()?.jobId != jobId) return
        val elapsedMs = locationCaptureBaseElapsedMs +
            (clock.monotonicMillis() - started).coerceAtLeast(0L)
        runCatching {
            logStore.appendLocations(
                jobId,
                listOf(
                    LocationObservation(
                        observation.observedAt,
                        elapsedMs,
                        observation.latitude,
                        observation.longitude,
                        observation.accuracyM,
                    ),
                ),
            )
        }.onSuccess {
            latestLocation = LiveLocation(
                latitude = observation.latitude,
                longitude = observation.longitude,
                accuracyM = observation.accuracyM,
                observedAt = observation.observedAt,
            )
            events.publish(LocationUpdated)
        }.onFailure(events::publish)
    }

    @Synchronized
    fun discoverySnapshot() = DiscoverySnapshot(bluetoothState, activeScanId, knownDevices)

    @Synchronized
    fun activeConnectionSnapshot(): ActiveConnectionSnapshot? {
        val deviceId = activeDeviceId ?: return null
        val savedDevice = if (trackingMode == TrackingMode.SAVED) productStore?.savedDevices()
            ?.firstOrNull { it.bluetoothIdentifier == deviceId.value }
        else null
        val activeSession = if (trackingMode == TrackingMode.SAVED) logStore.active()?.sessionId?.value?.let { sessionId ->
            runCatching { sessionStore?.session(sessionId) }.getOrNull()
        } else null
        return ActiveConnectionSnapshot(
            deviceId = deviceId,
            advertisedName = activeDeviceName,
            state = connectionState,
            trackingMode = trackingMode,
            savedDevice = if (trackingMode == TrackingMode.SAVED) currentSavedDevice ?: savedDevice else null,
            activeSession = activeSession,
            latestSample = latestPersistedSample,
            latestLocation = latestLocation,
            currentLimits = currentLimits,
            controllerRoute = controllerDiscovery.controllerRoute,
            controllerCandidates = controllerDiscovery.controllerCandidates,
            connectionIssue = controllerDiscovery.connectionIssue,
            bms = auxiliaryDiscovery.state,
            refloatControls = refloatMutations.snapshot(),
            controllerActions = controllerActionsSnapshot(),
        )
    }

    @Synchronized
    fun shutdownController() {
        val codec = protocol ?: throw RefloatMutationException()
        val route = controllerDiscovery.controllerRoute ?: throw RefloatMutationException()
        val firmware = controllerDiscovery.firmware ?: throw RefloatMutationException()
        val telemetry = latestRefloatTelemetry ?: throw RefloatMutationException()
        val context = ControllerShutdownContext(
            vescGeneration,
            connectionState == ConnectionState.READY,
            VescCompatibility.supportsShutdown(firmware),
            telemetry.rideState,
            telemetry.packageMode,
        )
        val actions = refloatMutations.authorizeShutdown(context)
        try {
            events.publish(actions)
            vescRequests.enqueueOneWay(
                codec.controllerShutdownRequest(route),
                VescExpectedResponse(VescCommand.SHUTDOWN.id),
                VescRequestPriority.RIDING_TELEMETRY,
                VescRequestContext.ControllerShutdown(route.toRequestRoute()),
                SHUTDOWN_INTENT_TIMEOUT_MS,
                vescGeneration,
            ) { _, outcome ->
                if (outcome != VescRequestCompletion.Sent) {
                    synchronized(this) { expireShutdownIntent(context.generation) }
                }
            }
            shutdownIntentTimer?.cancel()
            shutdownIntentTimer = refloatScheduler.schedule(SHUTDOWN_INTENT_TIMEOUT_MS) {
                synchronized(this) { expireShutdownIntent(context.generation) }
            }
        } catch (error: Exception) {
            refloatMutations.clearShutdown(context.generation)
            throw error
        }
    }

    @Synchronized
    fun firmwareSnapshot(): FirmwareRecord? = controllerDiscovery.firmware

    @Synchronized
    fun refloatPackageVersionSnapshot(): String? = controllerDiscovery.refloatPackageVersion

    fun setRefloatControl(control: RefloatBooleanControl, enabled: Boolean): RefloatControls {
        var immediate: RefloatControls? = null
        var waiter: RefloatMutationWaiter? = null
        synchronized(this) {
            val context = refloatMutationContext()
            when (val authorization = refloatMutations.authorizeSet(control, enabled, context)) {
                is RefloatMutationAuthorization.Idempotent -> immediate = authorization.controls
                is RefloatMutationAuthorization.Mutation -> {
                    waiter = RefloatMutationWaiter().also { refloatWaiter = it }
                    events.publish(authorization.controls)
                    submitRefloatMutation(authorization.mutation, context.route, context.generation)
                    armRefloatMutationTimeout(context.generation)
                }
            }
        }
        return immediate ?: waiter!!.await()
    }

    fun clearRefloatFatalError(): RefloatControls {
        var immediate: RefloatControls? = null
        var waiter: RefloatMutationWaiter? = null
        synchronized(this) {
            val context = refloatMutationContext()
            when (val authorization = refloatMutations.authorizeFatalClear(context)) {
                is RefloatMutationAuthorization.Idempotent -> immediate = authorization.controls
                is RefloatMutationAuthorization.Mutation -> {
                    waiter = RefloatMutationWaiter().also { refloatWaiter = it }
                    events.publish(authorization.controls)
                    submitRefloatMutation(authorization.mutation, context.route, context.generation)
                    armRefloatMutationTimeout(context.generation)
                }
            }
        }
        return immediate ?: waiter!!.await()
    }

    private fun startFirmwareDiscovery(deviceId: DeviceId) {
        if (protocol == null || activeDeviceId != deviceId) return
        vescGeneration = vescRequests.beginGeneration()
        val (generation, effect) = controllerDiscovery.begin()
        if (effect == ControllerDiscoveryEffect.None) return
        identificationTimer?.cancel()
        identificationTimer = scheduler?.schedule(IDENTIFICATION_TIMEOUT_MS) {
            identificationTimedOut(generation)
        }
        performDiscoveryEffect(effect)
    }

    private fun submitVescRequest(
        payload: ByteArray,
        expected: VescExpectedResponse,
        priority: VescRequestPriority,
        context: VescRequestContext,
        timeoutMs: Long,
        completion: (VescRequestCompletion) -> Unit = {},
    ) = submitVescRequestWithDiagnostic(
        payload, expected, priority, context, timeoutMs, null, completion,
    )

    private fun submitVescRequestWithDiagnostic(
        payload: ByteArray,
        expected: VescExpectedResponse,
        priority: VescRequestPriority,
        context: VescRequestContext,
        timeoutMs: Long,
        diagnosticAttempt: Int?,
        completion: (VescRequestCompletion) -> Unit,
    ) {
        if (priority == VescRequestPriority.CONTROLLER_IDENTIFICATION) {
            events.publish(identificationDiagnostic(context, diagnosticAttempt, "submitted"))
        }
        vescRequests.enqueue(payload, expected, priority, context, timeoutMs, vescGeneration) { _, outcome ->
            if (priority == VescRequestPriority.CONTROLLER_IDENTIFICATION) {
                events.publish(
                    identificationDiagnostic(context, diagnosticAttempt, requestOutcomeDiagnostic(outcome)),
                )
            }
            completion(outcome)
        }
    }

    private fun requestInitialFirmware(
        attempt: Int,
        identificationGeneration: Long,
        requestGeneration: VescRequestGeneration,
    ) {
        if (
            activeDeviceId == null ||
            connectionState != ConnectionState.IDENTIFYING ||
            controllerDiscovery.identificationGeneration != identificationGeneration ||
            vescGeneration != requestGeneration
        ) return
        val decoder = protocol ?: return
        try {
            submitVescRequestWithDiagnostic(
                decoder.firmwareRequest(ControllerRoute.Direct),
                VescExpectedResponse(VescCommand.GET_FIRMWARE_VERSION.id),
                VescRequestPriority.CONTROLLER_IDENTIFICATION,
                VescRequestContext.ControllerFirmware(VescRequestRoute.Direct),
                if (attempt == 1) {
                    INITIAL_IDENTIFICATION_ATTEMPT_TIMEOUT_MS
                } else {
                    IDENTIFICATION_TIMEOUT_MS - INITIAL_IDENTIFICATION_ATTEMPT_TIMEOUT_MS
                },
                diagnosticAttempt = attempt,
            ) { outcome ->
                if (outcome == VescRequestCompletion.TimedOut && attempt == 1) synchronized(this) {
                    requestInitialFirmware(2, identificationGeneration, requestGeneration)
                }
            }
        } catch (error: Exception) {
            failIdentification(
                ConnectionIssueCode.TRANSPORT_SETUP_FAILED,
                "Unable to request VESC firmware.",
            )
            events.publish(error)
        }
    }

    private fun handleFirmware(value: FirmwareRecord) {
        if (value.canId == null) {
            if (trackingMode == TrackingMode.SAVED) {
                currentSavedDevice = try {
                    val deviceId = activeDeviceId ?: return
                    productStore?.ensureSavedDevice(
                        deviceId,
                        activeDeviceName,
                        stableBoardIdentity(value.controllerUuid),
                        clock.now(),
                    )
                } catch (_: Exception) {
                    failIdentification(
                        ConnectionIssueCode.TRANSPORT_SETUP_FAILED,
                        "Unable to save the VESC device.",
                    )
                    return
                }
            }
            if (trackingMode == TrackingMode.SAVED) currentSavedDevice?.let { savedDevice ->
                val savedIds = runCatching {
                    productStore?.savedDevices().orEmpty()
                        .mapNotNull { it.bluetoothIdentifier?.let(::DeviceId) }
                        .toSet()
                }.getOrDefault(setOfNotNull(savedDevice.bluetoothIdentifier?.let(::DeviceId)))
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
        hasActiveSession = trackingMode == TrackingMode.SAVED && logStore.active() != null,
        route = sessionControllerRoute,
        controllerUuid = sessionControllerUuid,
    )

    private fun performDiscoveryEffect(effect: ControllerDiscoveryEffect) {
        when (effect) {
            is ControllerDiscoveryEffect.ProbeFirmware -> {
                if (activeDeviceId == null) return
                val decoder = protocol ?: return
                val route = effect.route
                if (route == ControllerRoute.Direct) {
                    requestInitialFirmware(
                        1,
                        controllerDiscovery.identificationGeneration,
                        vescGeneration,
                    )
                    return
                }
                if (route is ControllerRoute.Can) scheduleCanProbeTimeout(route.canId)
                try {
                    submitVescRequest(
                        decoder.firmwareRequest(route),
                        VescExpectedResponse(fail.acheron.floatview.protocol.vesc.VescCommand.GET_FIRMWARE_VERSION.id),
                        VescRequestPriority.CONTROLLER_IDENTIFICATION,
                        VescRequestContext.ControllerFirmware(route.toRequestRoute()),
                        if (route == ControllerRoute.Direct) IDENTIFICATION_TIMEOUT_MS else CAN_PROBE_TIMEOUT_MS,
                    )
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
                if (activeDeviceId == null) return
                val decoder = protocol ?: return
                try {
                    submitVescRequest(
                        decoder.canPingRequest(),
                        VescExpectedResponse(fail.acheron.floatview.protocol.vesc.VescCommand.PING_CAN.id),
                        VescRequestPriority.CONTROLLER_IDENTIFICATION,
                        VescRequestContext.CanPing,
                        IDENTIFICATION_TIMEOUT_MS,
                    )
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
                val savedDevice = if (trackingMode == TrackingMode.SAVED) currentSavedDevice else null
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
                if (activeDeviceId == null) return
                val decoder = protocol ?: return
                submitVescRequest(
                    decoder.refloatInfoRequest(effect.route),
                    VescExpectedResponse(
                        fail.acheron.floatview.protocol.vesc.VescCommand.CUSTOM_APP_DATA.id,
                        fail.acheron.floatview.protocol.refloat.RefloatCommand.INFO.id,
                    ),
                    VescRequestPriority.CONTROLLER_IDENTIFICATION,
                    VescRequestContext.RefloatInfo(effect.route.toRequestRoute()),
                    IDENTIFICATION_TIMEOUT_MS,
                )
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
        if (trackingMode == TrackingMode.SAVED && logStore.active() != null) {
            if (!controllerContinuityIsProven(route, selectedFirmware)) {
                failIdentification(
                    ConnectionIssueCode.CONTROLLER_CHANGED,
                    "The connected controller no longer matches the active session.",
                )
                return
            }
        } else if (trackingMode == TrackingMode.SAVED) {
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
        configureRefloatControls(selectedFirmware)
        protocol?.let { decoder ->
            runCatching {
                submitVescRequest(
                    decoder.motorConfigurationRequest(route),
                    VescExpectedResponse(fail.acheron.floatview.protocol.vesc.VescCommand.GET_MOTOR_CONFIGURATION.id),
                    VescRequestPriority.CONTROLLER_WORK,
                    VescRequestContext.CurrentLimits(route.toRequestRoute()),
                    CAN_PROBE_TIMEOUT_MS,
                )
            }.onFailure(events::publish)
        }
        telemetryPolling.start(
            deviceId,
            route,
            controllerDiscovery.refloatFamily ?: RefloatTelemetryFamily.DYNAMIC,
            idleProbeIntervalMs,
            postRideTailMs,
        )
        startAuxiliaryDiscovery()
    }

    private fun startAuxiliaryDiscovery() {
        if (protocol?.supportsBmsTelemetry != true) return
        val localIdentity = controllerDiscovery.transportFirmware ?: return
        val profile = currentSavedDevice?.savedDeviceId?.let { savedDeviceId ->
            runCatching { productStore?.boardProfile(savedDeviceId) }.getOrNull()
        }
        performAuxiliaryEffect(
            auxiliaryDiscovery.begin(
                localIdentity,
                controllerDiscovery.classifiedInventory.takeIf { controllerDiscovery.inventoryComplete },
                ControllerDiscoveryPolicy.preferredBmsCanId(profile?.valuesJson),
            ),
        )
    }

    private fun performAuxiliaryEffect(effect: AuxiliaryDiscoveryEffect) {
        if (connectionState != ConnectionState.READY) return
        val decoder = protocol ?: return
        when (effect) {
            AuxiliaryDiscoveryEffect.PingCan -> runCatching {
                events.publish("bms_discovery_scan_started")
                submitVescRequest(
                    decoder.canPingRequest(),
                    VescExpectedResponse(VescCommand.PING_CAN.id),
                    VescRequestPriority.AUXILIARY,
                    VescRequestContext.AuxiliaryCanPing,
                    CAN_SCAN_TIMEOUT_MS,
                ) { completion ->
                    if (completion == VescRequestCompletion.TimedOut) synchronized(this) {
                        auxiliaryDiscovery.markUnavailable()
                    }
                }
            }.onFailure { auxiliaryDiscovery.markUnavailable() }
            is AuxiliaryDiscoveryEffect.ProbeFirmware -> {
                events.publish(
                    "bms_discovery_probe:" +
                        bmsRouteDiagnostic((effect.route as? ControllerRoute.Can)?.canId),
                )
                val generation = auxiliaryDiscovery.generation
                val canId = (effect.route as? ControllerRoute.Can)?.canId
                runCatching {
                    submitVescRequest(
                        decoder.firmwareRequest(effect.route),
                        VescExpectedResponse(VescCommand.GET_FIRMWARE_VERSION.id),
                        VescRequestPriority.AUXILIARY,
                        VescRequestContext.AuxiliaryFirmware(effect.route.toRequestRoute()),
                        CAN_PROBE_TIMEOUT_MS,
                    ) { completion ->
                        if (completion == VescRequestCompletion.TimedOut && canId != null) synchronized(this) {
                            performAuxiliaryEffect(
                                auxiliaryDiscovery.probeTimedOut(canId, generation),
                            )
                        }
                    }
                }.onFailure { auxiliaryDiscovery.markUnavailable() }
            }
            is AuxiliaryDiscoveryEffect.RequestValues -> {
                events.publish(
                    "bms_values_requested:${bmsRouteDiagnostic((effect.connection as? BmsConnection.Can)?.canId)}",
                )
                requestBmsValues(effect.connection)
            }
            AuxiliaryDiscoveryEffect.None -> {
                events.publish("bms_discovery_complete:${auxiliaryDiscovery.state.status.wireValue}")
                events.publish(auxiliaryDiscovery.state)
            }
        }
    }

    private fun requestBmsValues(connection: BmsConnection) {
        if (connectionState != ConnectionState.READY) return
        val decoder = protocol ?: return
        bmsPollTimer?.cancel()
        bmsPollTimer = null
        val route = when (connection) {
            BmsConnection.Direct -> VescRequestRoute.Direct
            is BmsConnection.Can -> VescRequestRoute.Can(connection.canId)
        }
        runCatching {
            submitVescRequest(
                decoder.bmsValuesRequest(connection),
                VescExpectedResponse(VescCommand.BMS_GET_VALUES.id),
                VescRequestPriority.AUXILIARY,
                VescRequestContext.BmsValues(route),
                1_000,
            ) { completion ->
                synchronized(this) {
                    if (connectionState != ConnectionState.READY) return@synchronized
                    when (completion) {
                        VescRequestCompletion.TimedOut, VescRequestCompletion.QueueFull ->
                            recordBmsFailure()
                        is VescRequestCompletion.Response,
                        VescRequestCompletion.Sent,
                        -> Unit
                        VescRequestCompletion.Cancelled, VescRequestCompletion.StaleGeneration ->
                            return@synchronized
                    }
                    bmsPollTimer = scheduler?.schedule(1_000) {
                        synchronized(this) {
                            auxiliaryDiscovery.selectedConnection?.let(::requestBmsValues)
                        }
                    }
                }
            }
        }.onFailure { recordBmsFailure() }
    }

    private fun recordBmsFailure() {
        bmsFailureCount++
        if (bmsFailureCount >= 3) {
            auxiliaryDiscovery.markUnavailable()
            events.publish(auxiliaryDiscovery.state)
        }
    }

    /** Production hosts always provide a VESC codec and never take this path. */
    private fun completeWithoutProtocol() {
        val deviceId = activeDeviceId ?: return
        if (trackingMode == TrackingMode.SAVED && logStore.active() == null) {
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
        events.publish("connection_identification outcome=failed code=${code.name.lowercase()}")
        cancelIdentificationTimeout()
        cancelCanProbeTimeout()
        vescRequests.cancel(vescGeneration)
        telemetryPolling.cancelLayout()
        telemetryPolling.stop()
        controllerDiscovery.markFailure(code, message)
        if (trackingMode == TrackingMode.SAVED && code == ConnectionIssueCode.CONTROLLER_CHANGED) {
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
        if (trackingMode == TrackingMode.SAVED) logStore.active()?.let {
            events.publish(logStore.close(it.jobId, clock.now(), reason))
        }
        explicitDisconnect = true
        if (deviceId != null) bluetooth.disconnect(deviceId)
        activeDeviceId = null
        activeDeviceName = null
        latestPersistedSample = null
        latestLocation = null
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

    private fun configureRefloatControls(firmware: FirmwareRecord) {
        val profile = controllerDiscovery.refloatPackageVersion?.let(RefloatCompatibility::controlProfile)
        val access = VescCompatibility.accessMode(firmware)
        if (profile == null || access == null) { refloatProfile = null; return }
        refloatProfile = profile
        latestRefloatTelemetry = null
        events.publish(refloatMutations.configure(
            vescGeneration, profile,
            access == fail.acheron.floatview.protocol.vesc.VescAccessMode.READ_WRITE,
        ))
        scheduleRefloatLightingRefresh()
    }

    private fun requestRefloatLighting(route: ControllerRoute, profile: RefloatControlProfile) {
        if (connectionState != ConnectionState.READY || refloatMutations.snapshot()?.pendingAction != null) return
        runCatching {
            val snapshot = refloatMutations.snapshot()
            val command = RefloatControlCodec.lightingRequest(
                profile, null, current = RefloatLightingState(
                    snapshot?.ledsEnabled ?: false, snapshot?.headlightsEnabled ?: false,
                ),
            )
            submitVescRequest(
                VescFrameCodec.encode(VescCommandRouter.route(command, route)),
                VescExpectedResponse(VescCommand.CUSTOM_APP_DATA.id, command[2].toInt() and 0xff),
                VescRequestPriority.AUXILIARY, VescRequestContext.RefloatLighting(route.toRequestRoute()),
                CAN_PROBE_TIMEOUT_MS,
            )
        }.onFailure { events.publish("refloat_lighting_refresh_failed") }
    }

    private fun scheduleRefloatLightingRefresh() {
        refloatRefreshTimer?.cancel()
        refloatRefreshTimer = refloatScheduler.schedule(REFLOAT_REFRESH_MS) {
            synchronized(this) {
                val route = controllerDiscovery.controllerRoute
                val profile = refloatProfile
                if (route != null && profile != null) {
                    requestRefloatLighting(route, profile)
                    scheduleRefloatLightingRefresh()
                }
            }
        }
    }

    private fun refloatMutationContext(): RefloatMutationContext {
        val route = controllerDiscovery.controllerRoute
        val profile = refloatProfile
        val telemetry = latestRefloatTelemetry
        val firmware = controllerDiscovery.firmware
        if (connectionState != ConnectionState.READY || route == null || profile == null || telemetry == null || firmware == null) throw RefloatMutationException()
        return RefloatMutationContext(
            vescGeneration, route, profile, true,
            VescCompatibility.accessMode(firmware) == fail.acheron.floatview.protocol.vesc.VescAccessMode.READ_WRITE,
            telemetry.rideState, telemetry.packageState, telemetry.packageMode,
        )
    }

    private fun submitRefloatMutation(
        mutation: AuthorizedRefloatMutation,
        route: ControllerRoute,
        generation: VescRequestGeneration,
    ) {
        val payload = VescFrameCodec.encode(VescCommandRouter.route(mutation.payload, route))
        val completion: (VescScheduledRequest, VescRequestCompletion) -> Unit = { _, outcome ->
            when (outcome) {
                is VescRequestCompletion.Response, VescRequestCompletion.Sent -> Unit
                else -> synchronized(this) { failRefloatMutation(generation) }
            }
        }
        if (mutation.oneWay) {
            vescRequests.enqueueOneWay(
                payload, mutation.expectedResponse, VescRequestPriority.RIDING_TELEMETRY,
                VescRequestContext.RefloatControl(route.toRequestRoute()), REFLOAT_MUTATION_TIMEOUT_MS,
                generation, completion,
            )
        } else {
            vescRequests.enqueue(
                payload, mutation.expectedResponse, VescRequestPriority.RIDING_TELEMETRY,
                VescRequestContext.RefloatControl(route.toRequestRoute()), REFLOAT_MUTATION_TIMEOUT_MS,
                generation, completion,
            )
        }
    }

    private fun armRefloatMutationTimeout(generation: VescRequestGeneration) {
        refloatMutationTimer?.cancel()
        refloatMutationTimer = refloatScheduler.schedule(REFLOAT_MUTATION_TIMEOUT_MS) {
            synchronized(this) { failRefloatMutation(generation) }
        }
    }

    private fun failRefloatMutation(generation: VescRequestGeneration) {
        val updated = refloatMutations.failPending(generation) ?: return
        events.publish("refloat_control_confirmation_timeout")
        events.publish(updated)
        finishRefloatMutation(Result.failure(RefloatMutationException(unavailable = true)))
    }

    private fun finishRefloatMutation(result: Result<RefloatControls>) {
        refloatMutationTimer?.cancel(); refloatMutationTimer = null
        val waiter = refloatWaiter; refloatWaiter = null
        waiter?.finish(result)
    }

    private fun cancelRefloatControls() {
        refloatRefreshTimer?.cancel(); refloatRefreshTimer = null
        refloatMutationTimer?.cancel(); refloatMutationTimer = null
        shutdownIntentTimer?.cancel(); shutdownIntentTimer = null
        refloatProfile = null; latestRefloatTelemetry = null
        refloatMutations.cancel()
        finishRefloatMutation(Result.failure(RefloatMutationException(unavailable = true)))
    }

    private fun controllerActionsSnapshot(): ControllerActions {
        val supported = controllerDiscovery.firmware?.let(VescCompatibility::supportsShutdown) ?: false
        val telemetry = latestRefloatTelemetry
        val available = connectionState == ConnectionState.READY && supported &&
            telemetry?.rideState == RideState.IDLE && telemetry.packageMode != PackageMode.FLYWHEEL &&
            refloatMutations.snapshot()?.pendingAction == null
        return refloatMutations.controllerActions(available)
    }

    private fun expireShutdownIntent(generation: VescRequestGeneration) {
        val actions = refloatMutations.clearShutdown(generation) ?: return
        shutdownIntentTimer?.cancel(); shutdownIntentTimer = null
        events.publish("controller_shutdown_unconfirmed")
        events.publish(actions)
    }

    private fun transition(state: ConnectionState) {
        connectionState = state
        events.publish(state)
    }

    private fun resetFirmwareDiscovery() {
        vescGeneration = vescRequests.beginGeneration()
        telemetryPolling.cancelLayout()
        telemetryPolling.stop()
        controllerDiscovery.reset()
        auxiliaryDiscovery.reset()
        bmsPollTimer?.cancel()
        bmsPollTimer = null
        bmsFailureCount = 0
        currentLimits = null
        cancelRefloatControls()
        currentSavedDevice = null
        cancelIdentificationTimeout()
        cancelCanProbeTimeout()
        protocol?.reset()
    }

    private fun savedDeviceIdFor(deviceId: DeviceId): String? =
        runCatching {
            productStore?.savedDevices()?.firstOrNull { it.bluetoothIdentifier == deviceId.value }?.savedDeviceId
        }.getOrNull()

    private fun stableBoardIdentity(value: ByteArray?): String? {
        val meaningful = ControllerDiscoveryPolicy.meaningfulUuid(value) ?: return null
        val input = ByteArray("float-view-board-v1\u0000".toByteArray().size + meaningful.size).also {
            val prefix = "float-view-board-v1\u0000".toByteArray()
            prefix.copyInto(it)
            meaningful.copyInto(it, prefix.size)
        }
        val digest = MessageDigest.getInstance("SHA-256").digest(input)
        return "vesc-v1-" + digest.joinToString("") { byte -> "%02x".format(byte.toInt() and 0xff) }
    }

    private fun permissionAllowsUse(state: BluetoothPermissionState): Boolean =
        state == BluetoothPermissionState.GRANTED ||
            state == BluetoothPermissionState.NOT_REQUIRED

    private fun permissionIsTerminal(state: BluetoothPermissionState): Boolean =
        state == BluetoothPermissionState.DENIED ||
            state == BluetoothPermissionState.RESTRICTED

    private fun bmsRouteDiagnostic(canId: Int?): String = canId?.let { "can=$it" } ?: "direct"

    private fun bmsHardwareDiagnostic(value: String?): String =
        value.orEmpty().take(32).map { character ->
            if (character.isLetterOrDigit() || character == '-' || character == '_') character else '_'
        }.joinToString("").ifEmpty { "unknown" }

    private fun identificationDiagnostic(
        context: VescRequestContext,
        attempt: Int?,
        outcome: String,
    ): String {
        val stage = when (context) {
            is VescRequestContext.ControllerFirmware -> "firmware"
            VescRequestContext.CanPing -> "can_ping"
            is VescRequestContext.RefloatInfo -> "refloat_info"
            is VescRequestContext.RefloatLayout -> "telemetry_layout"
            else -> "other"
        }
        val route = when (val value = context.route) {
            VescRequestRoute.Direct -> "direct"
            is VescRequestRoute.Can -> "can:${value.canId}"
            null -> "none"
        }
        return "connection_identification stage=$stage route=$route " +
            "attempt=${attempt ?: 1} outcome=$outcome"
    }

    private fun requestOutcomeDiagnostic(outcome: VescRequestCompletion): String = when (outcome) {
        is VescRequestCompletion.Response -> "response"
        VescRequestCompletion.Sent -> "sent"
        VescRequestCompletion.TimedOut -> "timed_out"
        VescRequestCompletion.QueueFull -> "queue_full"
        VescRequestCompletion.Cancelled -> "cancelled"
        VescRequestCompletion.StaleGeneration -> "stale_generation"
    }

    private companion object {
        const val MAX_RECONNECT_BACKOFF_MS = 30_000L
        const val DISCOVERY_EXPIRY_MS = 120_000L
        const val IDENTIFICATION_TIMEOUT_MS = 10_000L
        const val INITIAL_IDENTIFICATION_ATTEMPT_TIMEOUT_MS = 2_000L
        // COMM_PING_CAN can synchronously probe all 255 CAN IDs at 10 ms per ID.
        const val CAN_SCAN_TIMEOUT_MS = 10_000L
        const val CAN_PROBE_TIMEOUT_MS = 1_000L
        const val REFLOAT_REFRESH_MS = 1_000L
        const val REFLOAT_MUTATION_TIMEOUT_MS = 3_000L
        // Android can surface controller power loss only after the BLE link
        // supervision timeout (GATT status 8), well after the command write.
        const val SHUTDOWN_INTENT_TIMEOUT_MS = 30_000L
    }
}

private class RefloatMutationWaiter {
    private val latch = CountDownLatch(1)
    @Volatile private var result: Result<RefloatControls>? = null

    @Synchronized fun finish(value: Result<RefloatControls>) {
        if (result != null) return
        result = value
        latch.countDown()
    }

    fun await(): RefloatControls {
        if (!latch.await(3_250, TimeUnit.MILLISECONDS)) throw RefloatMutationException(unavailable = true)
        return result!!.getOrThrow()
    }
}
