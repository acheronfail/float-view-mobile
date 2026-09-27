package fail.acheron.floatview.application

import android.util.Base64
import java.security.MessageDigest
import java.time.Instant
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.json.JSONArray
import org.json.JSONObject

class ApplicationService(
    private val bluetooth: BluetoothPort,
    private val clock: ClockPort,
    private val logStore: LogStorePort,
    private val productStore: ProductStorePort? = null,
    private val sessionStore: SessionStorePort? = null,
    private val scheduler: SchedulerPort? = null,
    samplingSettings: SamplingSettings = SamplingSettings(),
    private val locationProvider: LocationProviderPort? = null,
    private val events: EventSink,
) {
    private val rustRuntime = RustApplicationRuntime()
    private var rustGeneration = 0L
    private val rustDeadlines = mutableMapOf<Long, CancellationPort>()
    private val rustTimers = mutableMapOf<Long, CancellationPort>()
    private val rustTimerIntervals = mutableMapOf<Long, Long>()
    private var rustSessionAttached = false
    private var rustControllerRoute: ControllerRoute? = null
    private var rustControllerCandidates = emptyList<ControllerCandidate>()
    private var rustConnectionIssue: ConnectionIssue? = null
    private var rustFirmware: FirmwareRecord? = null
    private var rustRefloatPackageVersion: String? = null
    private var rustControls: RefloatControls? = null
    private var rustControllerActions = ControllerActions()
    private var rustBmsState = BmsState()
    private var rustConfigurationSummary = RefloatConfigurationSummary(
        RefloatConfigurationAvailability.UNAVAILABLE, false, null, null, null, false, false, null,
    )
    private var rustConfigurationSchema: RefloatConfigurationSchema? = null
    private var rustConfigurationCurrent: RefloatConfigurationValues? = null
    private var rustConfigurationDefaults: RefloatConfigurationValues? = null
    private var rustSessionResult: JSONObject? = null
    private var samplingSettings = samplingSettings
    private var reconnectDurationMs = samplingSettings.reconnectDurationMs
    private var idleProbeIntervalMs = samplingSettings.idleProbeIntervalMs
    private var explicitDisconnect = false
    private var scanSequence = 0
    private val knownDevicesById = linkedMapOf<DeviceId, DiscoveredDevice>()
    private var discoveryExpiryTimer: CancellationPort? = null
    private var discoveryExpiryGeneration = 0L
    private var deviceKind: DeviceKind? = null
    private var trackingMode = TrackingMode.SAVED
    private var currentSavedDevice: SavedDeviceRecord? = null
    private var sessionControllerRoute: ControllerRoute? = null
    private var sessionControllerUuid: ByteArray? = null
    private var latestPersistedSample: TelemetrySample? = null
    private var latestLocation: LiveLocation? = null
    private var locationCaptureStartedMonotonicMs: Long? = null
    private var locationCaptureBaseElapsedMs = 0L
    private var locationCaptureActive = false
    private var currentLimits: ControllerCurrentLimits? = null
    private var refloatWaiter: RefloatMutationWaiter? = null
    private var refloatConfigurationWaiter: RefloatConfigurationRequestWaiter? = null
    private var refloatConfigurationObservedPending = false
    private var reconnectTimer: CancellationPort? = null
    private var reconnectStartedAtMs: Long? = null
    private var reconnectAttempt = 0

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

    init {
        val savedIds = runCatching {
            productStore?.savedDevices().orEmpty().mapNotNull {
                it.bluetoothIdentifier?.let(::DeviceId)
            }.toSet()
        }.getOrDefault(emptySet())
        bluetooth.configureSavedDeviceIds(savedIds)
    }

    @Synchronized
    fun requestBluetoothPermission(): BluetoothState {
        bluetooth.requestPermission()
        bluetoothState = bluetooth.currentState()
        return bluetoothState
    }

    fun savedDevices(): List<SavedDeviceRecord> = product().savedDevices()

    fun updateSavedDevice(
        savedDeviceId: String,
        expectedRevision: Int,
        customDisplayName: String?,
    ): SavedDeviceRecord = product()
        .updateSavedDevice(savedDeviceId, expectedRevision, customDisplayName, clock.now())
        .also(events::publish)

    @Synchronized
    fun deleteSavedDevice(savedDeviceId: String) {
        if (activeConnectionSnapshot()?.savedDevice?.savedDeviceId == savedDeviceId) {
            throw ProductStoreException.SavedDeviceActive
        }
        product().deleteSavedDevice(savedDeviceId)
        events.publish("saved device deleted")
    }

    fun boardProfile(savedDeviceId: String): BoardProfileRecord = product().boardProfile(savedDeviceId)

    @Synchronized
    fun updateBoardProfile(
        savedDeviceId: String,
        expectedRevision: Int,
        patchJson: String,
    ): BoardProfileRecord = product()
        .updateBoardProfile(savedDeviceId, expectedRevision, patchJson, clock.now())
        .also(events::publish)

    @Synchronized
    fun selectController(deviceId: DeviceId, canId: Int) {
        check(activeDeviceId == deviceId && connectionState == ConnectionState.SELECTING_CONTROLLER &&
            rustControllerCandidates.any { it.canId == canId })
        if (trackingMode == TrackingMode.SAVED) {
            product().selectController(
                requireNotNull(currentSavedDevice).savedDeviceId, canId, null, clock.now(),
            ).also(events::publish)
        }
        transition(ConnectionState.IDENTIFYING)
        performRustCommand(json(
            "command" to "select_controller", "generation" to rustGeneration,
            "can_id" to canId, "now_ms" to clock.monotonicMillis(),
        ))
    }

    fun settings(): SettingsRecord = product().settings()

    @Synchronized
    fun locationState(): LocationProviderState {
        if (!runCatching { settings().locationRecordingEnabled }.getOrDefault(false)) {
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
        product().updateSettings(expectedRevision, patchJson).also { value ->
            idleProbeIntervalMs = value.idleProbeIntervalMs
            reconnectDurationMs = value.reconnectDurationMs
            samplingSettings = SamplingSettings(
                idleProbeIntervalMs = value.idleProbeIntervalMs,
                idlePersistIntervalMs = value.idlePersistIntervalMs,
                ridingPersistIntervalMs = value.ridingPersistIntervalMs,
                postRideTailMs = value.postRideTailMs,
                reconnectDurationMs = value.reconnectDurationMs,
            )
            runCatching { performRustCommand(json(
                "command" to "update_settings", "generation" to rustGeneration,
                "idle_interval_ms" to value.idleProbeIntervalMs,
                "post_ride_tail_ms" to value.postRideTailMs,
                "now_ms" to clock.monotonicMillis(),
            )) }
            if (!value.locationRecordingEnabled) stopLocationCapture() else locationProvider?.requestPermission()
            events.publish(value)
        }

    fun sessions(query: SessionListQuery): SessionPage = sessions().sessions(query)
    fun session(sessionId: String): SessionSummaryRecord = sessions().session(sessionId)
    fun sessionSnapshot(sessionId: String): SessionPublishedSnapshotRecord = sessions().sessionSnapshot(sessionId)
    fun maintenanceJob(): SessionMaintenanceJobRecord? = sessions().maintenanceJob()
    fun saveMaintenanceJob(job: SessionMaintenanceJobRecord) = sessions().saveMaintenanceJob(job)
    fun cancelMaintenanceJob(operationId: String): SessionMaintenanceJobRecord? =
        sessions().cancelMaintenanceJob(operationId)
    fun deleteSession(sessionId: String) = sessions().deleteSession(sessionId)
    fun cancelPendingDelete(operationId: String): Boolean = sessions().cancelPendingDelete(operationId)
    fun exportSession(sessionId: String, format: String): SessionExportRecord =
        sessions().exportSession(sessionId, format)
    fun importSessionArchive(data: ByteArray): String = sessions().importSessionArchive(data)

    fun prunePreview(sessionId: String, targetResolutionSeconds: Long) =
        rustPrune(sessionId, targetResolutionSeconds, false)
    fun pruneSession(sessionId: String, targetResolutionSeconds: Long): SessionPrunePreviewRecord {
        return rustPrune(sessionId, targetResolutionSeconds, true).also { sessions().reconcileSession(sessionId) }
    }
    @Synchronized
    fun startDiscovery(): String {
        check(bluetoothState.radio == BluetoothRadioState.POWERED_ON)
        check(bluetoothState.scanPermission in setOf(
            BluetoothPermissionState.GRANTED, BluetoothPermissionState.NOT_REQUIRED,
        ))
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
        require(scanId == active)
        bluetooth.stopDiscovery()
        activeScanId = null
        events.publish(discoverySnapshot())
    }

    @Synchronized
    fun connect(deviceId: DeviceId, trackingMode: TrackingMode = TrackingMode.SAVED) {
        require(knownDevicesById.containsKey(deviceId))
        check(activeDeviceId == null || (activeDeviceId == deviceId && connectionState == ConnectionState.FAILED))
        if (activeDeviceId == deviceId && connectionState == ConnectionState.FAILED) {
            require(this.trackingMode == trackingMode)
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
        }
        explicitDisconnect = false
        cancelReconnect()
        latestPersistedSample = null
        latestLocation = null
        performRustCommand(json("command" to "reset", "now_ms" to clock.monotonicMillis()))
        transition(ConnectionState.CONNECTING)
        bluetooth.connect(deviceId)
    }

    @Synchronized
    fun onBluetoothEvent(event: BluetoothEvent) {
        when (event) {
            is BluetoothEvent.StateChanged -> handleBluetoothState(event.state)
            is BluetoothEvent.DeviceFound -> {
                val device = DiscoveredDevice(
                    event.deviceId, event.advertisedName, event.rssi, clock.now(),
                    savedDeviceIdFor(event.deviceId),
                )
                knownDevicesById[event.deviceId] = device
                scheduleDiscoveryExpiry()
                events.publish(device)
                events.publish(discoverySnapshot())
            }
            is BluetoothEvent.ScanFailed -> {
                activeScanId = null
                events.publish(event.reason)
            }
            is BluetoothEvent.ConnectionChanged -> handleConnection(event.deviceId, event.state)
            is BluetoothEvent.Notification -> if (event.deviceId == activeDeviceId) {
                performRustCommand(json(
                    "command" to "receive_bytes", "generation" to rustGeneration,
                    "bytes" to event.bytes.unsigned(), "now_ms" to clock.monotonicMillis(),
                    "observed_at_epoch_ms" to clock.now().toEpochMilli(),
                ))
            }
        }
    }

    @Synchronized
    fun disconnect() {
        val deviceId = activeDeviceId ?: return
        transition(ConnectionState.DISCONNECTING)
        explicitDisconnect = true
        cancelReconnect()
        stopLocationCapture()
        if (trackingMode == TrackingMode.SAVED) {
            logStore.active()?.let { active ->
                performRustCommand(json("command" to "detach_session"))
                rustSessionAttached = false
                events.publish(logStore.close(active.jobId, clock.now(), "explicit_disconnect"))
            }
        }
        cancelRustTimers()
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
        // Rust commits telemetry before publishing it; this method remains for test/software hosts.
        if (!rustSessionAttached) logStore.append(active.jobId, listOf(sample))
        latestPersistedSample = sample
        events.publish(sample)
    }

    @Synchronized
    fun discoverySnapshot() = DiscoverySnapshot(bluetoothState, activeScanId, knownDevices)

    @Synchronized
    fun activeConnectionSnapshot(): ActiveConnectionSnapshot? {
        val deviceId = activeDeviceId ?: return null
        val activeSession = if (trackingMode == TrackingMode.SAVED) {
            logStore.active()?.let { runCatching { session(it.sessionId.value) }.getOrNull() }
        } else null
        return ActiveConnectionSnapshot(
            deviceId, activeDeviceName, connectionState, deviceKind, trackingMode,
            if (trackingMode == TrackingMode.SAVED) currentSavedDevice else null,
            activeSession, latestPersistedSample, latestLocation, currentLimits,
            rustControllerRoute, rustControllerCandidates, rustConnectionIssue,
            rustBmsState, rustControls, rustControllerActions,
        )
    }

    @Synchronized
    fun shutdownController() = performRustCommand(json(
        "command" to "shutdown_controller", "generation" to rustGeneration,
        "now_ms" to clock.monotonicMillis(),
    ))

    fun firmwareSnapshot(): FirmwareRecord? = synchronized(this) { rustFirmware }
    fun refloatPackageVersionSnapshot(): String? = synchronized(this) { rustRefloatPackageVersion }
    fun refloatConfigurationSummarySnapshot() = synchronized(this) { rustConfigurationSummary }
    fun refloatConfigurationSchemaSnapshot() = synchronized(this) { rustConfigurationSchema }
    fun refloatConfigurationValuesSnapshot(defaults: Boolean) = synchronized(this) {
        if (defaults) rustConfigurationDefaults else rustConfigurationCurrent
    }

    fun setRefloatControl(control: RefloatBooleanControl, enabled: Boolean): RefloatControls {
        val waiter = synchronized(this) {
            RefloatMutationWaiter().also {
                refloatWaiter = it
                performRustCommand(json(
                    "command" to "set_control", "generation" to rustGeneration,
                    "control" to control.wireValue, "enabled" to enabled,
                    "now_ms" to clock.monotonicMillis(),
                ))
            }
        }
        synchronized(this) { rustControls }?.takeIf { it.pendingAction == null }?.let { return it }
        return waiter.await()
    }

    fun clearRefloatFatalError(): RefloatControls {
        val waiter = synchronized(this) {
            RefloatMutationWaiter().also {
                refloatWaiter = it
                performRustCommand(json(
                    "command" to "clear_fatal_error", "generation" to rustGeneration,
                    "now_ms" to clock.monotonicMillis(),
                ))
            }
        }
        synchronized(this) { rustControls }?.takeIf { it.pendingAction == null }?.let { return it }
        return waiter.await()
    }

    fun readRefloatConfiguration(defaults: Boolean): RefloatConfigurationValues {
        val waiter = synchronized(this) {
            RefloatConfigurationRequestWaiter().also {
                refloatConfigurationWaiter = it
                refloatConfigurationObservedPending = false
                performRustCommand(json(
                    "command" to "read_configuration", "generation" to rustGeneration,
                    "defaults" to defaults, "now_ms" to clock.monotonicMillis(),
                ))
            }
        }
        waiter.await()
        return synchronized(this) {
            if (defaults) rustConfigurationDefaults else rustConfigurationCurrent
        } ?: throw RefloatConfigurationServiceException(RefloatConfigurationServiceError.INVALID_STATE)
    }

    fun writeRefloatConfiguration(
        packageVersion: String, fingerprint: String, wireSignature: Long,
        baseRevision: Long, values: Map<String, RefloatConfigurationValue>,
    ): RefloatConfigurationValues = writeRefloatConfiguration(
        "write", packageVersion, fingerprint, wireSignature, baseRevision, values,
    )

    private fun writeRefloatConfiguration(
        mutation: String, packageVersion: String, fingerprint: String, wireSignature: Long,
        baseRevision: Long, values: Map<String, RefloatConfigurationValue>,
    ): RefloatConfigurationValues {
        val waiter = synchronized(this) {
            RefloatConfigurationRequestWaiter().also {
                refloatConfigurationWaiter = it
                refloatConfigurationObservedPending = false
                performRustCommand(json(
                    "command" to "write_configuration", "generation" to rustGeneration,
                    "mutation" to mutation, "package_version" to packageVersion,
                    "fingerprint" to fingerprint, "wire_signature" to wireSignature,
                    "base_revision" to baseRevision, "values" to configurationValuesJson(values),
                    "now_ms" to clock.monotonicMillis(),
                ))
            }
        }
        waiter.await()
        return synchronized(this) { rustConfigurationCurrent }
            ?: throw RefloatConfigurationServiceException(RefloatConfigurationServiceError.INVALID_STATE)
    }

    fun refloatTunes(
        identity: RefloatConfigurationIdentityRecord, limit: Int, cursor: String?,
    ) = product().refloatConfigurations(identity, limit, cursor)

    fun refloatTune(configurationId: String) = product().refloatConfiguration(configurationId)

    fun createRefloatTune(
        name: String, description: String?, identity: RefloatConfigurationIdentityRecord,
        sourceRevision: Long, values: Map<String, RefloatConfigurationValue>,
    ): RefloatConfigurationRecord {
        validateRefloatTuneValues(identity, sourceRevision, values)
        return product().createRefloatConfiguration(
            name, description, identity, configurationValuesString(values), clock.now(),
        )
    }

    fun updateRefloatTune(
        configurationId: String, expectedRevision: Int, name: String,
        description: String?, values: Map<String, RefloatConfigurationValue>,
    ): RefloatConfigurationRecord {
        val record = product().refloatConfiguration(configurationId)
        validateRefloatTuneValues(record.identity, rustConfigurationCurrent?.revision, values)
        return product().updateRefloatConfiguration(
            configurationId, expectedRevision, name, description,
            configurationValuesString(values), clock.now(),
        )
    }

    fun deleteRefloatTune(configurationId: String, expectedRevision: Int) =
        product().deleteRefloatConfiguration(configurationId, expectedRevision)

    fun applyRefloatTune(
        configurationId: String, expectedRevision: Int, baseRevision: Long,
    ): RefloatConfigurationValues {
        val record = product().refloatConfiguration(configurationId)
        if (record.revision != expectedRevision) {
            throw ProductStoreException.RefloatConfigurationRevisionConflict(record.revision)
        }
        return writeRefloatConfiguration(
            "tune_apply", record.identity.packageVersion, record.identity.schemaFingerprint,
            record.identity.wireSignature, baseRevision,
            configurationValues(JSONObject(record.valuesJson)),
        )
    }

    private fun validateRefloatTuneValues(
        identity: RefloatConfigurationIdentityRecord,
        sourceRevision: Long?,
        values: Map<String, RefloatConfigurationValue>,
    ) {
        val summary = rustConfigurationSummary
        val valid = identity.packageName == "refloat" &&
            summary.packageVersion == identity.packageVersion &&
            summary.schemaFingerprint == identity.schemaFingerprint &&
            summary.wireSignature == identity.wireSignature &&
            sourceRevision != null && rustConfigurationCurrent?.revision == sourceRevision &&
            rustConfigurationSchema != null && values.values.all {
                it !is RefloatConfigurationValue.Decimal || it.value.isFinite()
            }
        if (!valid) throw RefloatConfigurationServiceException(RefloatConfigurationServiceError.INVALID_STATE)
    }

    private fun handleBluetoothState(state: BluetoothState) {
        bluetoothState = state
        if ((state.radio != BluetoothRadioState.POWERED_ON || !permissionAllowsUse(state.scanPermission)) &&
            activeScanId != null
        ) {
            bluetooth.stopDiscovery()
            activeScanId = null
        }
        if (permissionIsTerminal(state.connectPermission) && activeDeviceId != null && !explicitDisconnect) {
            finishReconnect("bluetooth_permission_denied")
        } else if (state.radio != BluetoothRadioState.POWERED_ON && activeDeviceId != null && !explicitDisconnect) {
            beginReconnect()
        }
        events.publish(state)
    }

    private fun handleConnection(deviceId: DeviceId, state: ConnectionState) {
        if (deviceId != activeDeviceId) return
        if (state in setOf(ConnectionState.FAILED, ConnectionState.DISCONNECTED) &&
            rustControllerActions.pendingAction == ControllerPendingAction.SHUTDOWN
        ) {
            finishReconnect("controller_shutdown")
            return
        }
        if (state == ConnectionState.READY) {
            transition(ConnectionState.IDENTIFYING)
            beginRustApplication()
            return
        }
        if (state in setOf(ConnectionState.FAILED, ConnectionState.DISCONNECTED) &&
            !explicitDisconnect && trackingMode in setOf(TrackingMode.SAVED, TrackingMode.GUEST)
        ) {
            beginReconnect()
            return
        }
        if (state == ConnectionState.DISCONNECTED) {
            cancelReconnect()
            activeDeviceId = null
            activeDeviceName = null
            latestPersistedSample = null
            latestLocation = null
        }
        transition(state)
    }

    private fun beginReconnect() {
        if (reconnectDurationMs <= 0) return finishReconnect("unexpected_disconnect")
        if (reconnectStartedAtMs == null) {
            reconnectStartedAtMs = clock.monotonicMillis()
            reconnectAttempt = 0
        }
        transition(ConnectionState.RECONNECTING)
        scheduleReconnectAttempt()
    }

    private fun scheduleReconnectAttempt() {
        val activeScheduler = scheduler ?: return finishReconnect("reconnect_unavailable")
        if (reconnectTimer != null || reconnectStartedAtMs == null) return
        val remaining = reconnectDurationMs - (clock.monotonicMillis() - (reconnectStartedAtMs ?: 0))
        if (remaining <= 0) return finishReconnect("reconnect_timeout")
        val delay = minOf(1_000L shl minOf(reconnectAttempt, 5), MAX_RECONNECT_BACKOFF_MS, remaining)
        reconnectTimer = activeScheduler.schedule(delay, ::performReconnectAttempt)
    }

    @Synchronized
    private fun performReconnectAttempt() {
        reconnectTimer = null
        val deviceId = activeDeviceId ?: return
        if (connectionState != ConnectionState.RECONNECTING) return
        if (clock.monotonicMillis() - (reconnectStartedAtMs ?: 0) >= reconnectDurationMs) {
            return finishReconnect("reconnect_timeout")
        }
        if (bluetoothState.radio != BluetoothRadioState.POWERED_ON) return scheduleReconnectAttempt()
        reconnectAttempt += 1
        bluetooth.connect(deviceId)
    }

    private fun finishReconnect(reason: String) {
        val deviceId = activeDeviceId
        cancelReconnect()
        if (rustSessionAttached) {
            runCatching { performRustCommand(json("command" to "detach_session")) }
            rustSessionAttached = false
        }
        if (trackingMode == TrackingMode.SAVED) {
            logStore.active()?.let { active ->
                runCatching { events.publish(logStore.close(active.jobId, clock.now(), reason)) }
            }
        }
        stopLocationCapture()
        cancelRustTimers()
        deviceId?.let(bluetooth::disconnect)
        activeDeviceId = null
        activeDeviceName = null
        currentSavedDevice = null
        latestPersistedSample = null
        latestLocation = null
        rustConnectionIssue = null
        transition(ConnectionState.DISCONNECTED)
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

    private fun scheduleDiscoveryExpiry() {
        discoveryExpiryTimer?.cancel()
        val generation = ++discoveryExpiryGeneration
        discoveryExpiryTimer = scheduler?.schedule(DISCOVERY_EXPIRY_MS) {
            synchronized(this) {
                if (generation == discoveryExpiryGeneration) {
                    knownDevicesById.clear()
                    events.publish(discoverySnapshot())
                }
            }
        }
    }

    private fun savedDeviceIdFor(deviceId: DeviceId): String? = runCatching {
        productStore?.savedDevices()?.firstOrNull { it.bluetoothIdentifier == deviceId.value }?.savedDeviceId
    }.getOrNull()

    private fun product() = requireNotNull(productStore) { "Product store is unavailable" }
    private fun sessions() = requireNotNull(sessionStore) { "Session store is unavailable" }

    private fun updateLocationCapture(rideState: RideState) {
        val provider = locationProvider
        val enabled = runCatching { productStore?.settings()?.locationRecordingEnabled }.getOrDefault(false)
        val active = logStore.active()
        if (trackingMode != TrackingMode.SAVED || provider == null || enabled != true ||
            rideState != RideState.RIDING || active == null ||
            provider.state().authorization !in setOf(
                LocationProviderAuthorization.AUTHORIZED,
                LocationProviderAuthorization.FOREGROUND_ONLY,
            )
        ) {
            stopLocationCapture()
            return
        }
        if (locationCaptureActive) return
        locationCaptureActive = true
        locationCaptureStartedMonotonicMs = clock.monotonicMillis()
        locationCaptureBaseElapsedMs = latestPersistedSample?.elapsedMs ?: 0
        provider.startUpdates { observation -> ingestLocation(observation, active.jobId) }
    }

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
        val elapsed = locationCaptureBaseElapsedMs + maxOf(0, clock.monotonicMillis() - started)
        runCatching {
            performRustCommand(json(
                "command" to "append_locations",
                "locations" to listOf(json(
                    "observed_at_epoch_ms" to observation.observedAt.toEpochMilli(),
                    "elapsed_ms" to elapsed, "latitude" to observation.latitude,
                    "longitude" to observation.longitude, "accuracy_m" to observation.accuracyM,
                )),
            ))
            latestLocation = LiveLocation(
                observation.latitude, observation.longitude, observation.accuracyM, observation.observedAt,
            )
            events.publish(LocationUpdated)
        }.onFailure { events.publish(it) }
    }

    internal fun sessionPath(sessionId: String) = sessions().sessionPath(sessionId)

    private fun rustSessionQuery(command: JSONObject): JSONObject = synchronized(this) {
        rustSessionResult = null
        performRustCommand(command)
        val result = rustSessionResult ?: throw runtimeFailure()
        rustSessionResult = null
        result
    }

    private fun rustPrune(
        sessionId: String, targetResolutionSeconds: Long, commit: Boolean,
    ): SessionPrunePreviewRecord {
        val value = rustSessionQuery(json(
            "command" to "session_prune", "path" to sessionPath(sessionId),
            "target_resolution_seconds" to targetResolutionSeconds, "commit" to commit,
            "committed_at_epoch_ms" to clock.now().toEpochMilli(),
        ))
        val skipped = value.optBoolean("skipped")
        return SessionPrunePreviewRecord(
            value.requireLong("recorded_sample_count"), value.requireLong("retained_sample_count"),
            value.requireLong("estimated_reclaimable_bytes"), skipped,
            if (skipped) "already_at_or_below_target" else null,
        )
    }

    private fun beginRustApplication() {
        val saved = runCatching {
            productStore?.savedDevices()?.firstOrNull { it.bluetoothIdentifier == activeDeviceId?.value }
        }.getOrNull()
        currentSavedDevice = saved
        val profile = saved?.let { runCatching { productStore?.boardProfile(it.savedDeviceId) }.getOrNull() }
        val hint = saved?.let { runCatching { productStore?.controllerRoutingHint(it.savedDeviceId) }.getOrNull() }
        performRustCommand(json(
            "command" to "begin", "now_ms" to clock.monotonicMillis(),
            "idle_interval_ms" to idleProbeIntervalMs,
            "post_ride_tail_ms" to samplingSettings.postRideTailMs,
            "preferred_controller_can_id" to profile?.valuesJson?.let { preferredCanId(it, "preferredCanId") },
            "preferred_bms_can_id" to profile?.valuesJson?.let { preferredCanId(it, "preferredBmsCanId") },
            "routing_hint_can_id" to hint?.canId,
            "routing_hint_controller_uuid" to hint?.controllerUuid?.unsigned(),
            "has_active_session" to (trackingMode == TrackingMode.SAVED && logStore.active() != null),
            "continuity_can_id" to routeCanId(sessionControllerRoute),
            "continuity_controller_uuid" to sessionControllerUuid?.unsigned(),
        ))
    }

    private fun performRustCommand(command: JSONObject) {
        val result = try {
            rustRuntime.apply(command)
        } catch (error: RustApplicationRuntimeException) {
            events.publish("rust_runtime_apply_failure_${command.optString("command", "invalid")}")
            throw when {
                error.reason == RustApplicationRuntimeError.INVALID_STATE &&
                    command.optString("command") in setOf("set_control", "clear_fatal_error", "shutdown_controller") ->
                    RefloatMutationException()
                error.reason == RustApplicationRuntimeError.INVALID_STATE &&
                    command.optString("command") in setOf("read_configuration", "write_configuration") ->
                    RefloatConfigurationServiceException(RefloatConfigurationServiceError.INVALID_STATE)
                error.reason == RustApplicationRuntimeError.CONFLICT ->
                    RefloatConfigurationServiceException(RefloatConfigurationServiceError.CONFLICT)
                error.reason == RustApplicationRuntimeError.UNAVAILABLE ->
                    RefloatConfigurationServiceException(RefloatConfigurationServiceError.UNAVAILABLE)
                else -> error
            }
        }
        rustGeneration = result.generation
        result.effects.forEach { effect ->
            try {
                performRustEffect(effect)
            } catch (error: Exception) {
                events.publish("rust_runtime_effect_failure_${effect.optString("effect", "invalid")}")
                throw error
            }
        }
    }

    private fun performRustEffect(effect: JSONObject) {
        when (effect.string("effect") ?: throw runtimeFailure()) {
            "write_bytes" -> bluetooth.write(
                activeDeviceId ?: throw runtimeFailure(),
                effect.intList("bytes")?.map(Int::toByte)?.toByteArray() ?: throw runtimeFailure(),
            )
            "arm_deadline" -> armRustTimer(effect, repeating = false, deadline = true)
            "replace_timer" -> armRustTimer(effect, repeating = true, deadline = false)
            "cancel_deadline" -> {
                val token = effect.long("token") ?: throw runtimeFailure()
                rustDeadlines.remove(token)?.cancel()
            }
            "cancel_timer" -> {
                val token = effect.long("token") ?: throw runtimeFailure()
                rustTimers.remove(token)?.cancel()
                rustTimerIntervals.remove(token)
            }
            "clear_routing_hint" -> currentSavedDevice?.savedDeviceId?.let {
                runCatching { productStore?.clearControllerRoutingHint(it) }
            }
            "store_routing_hint" -> storeRustRoutingHint(effect)
            "prepare_activation" -> prepareRustActivation(effect)
            "publish_state" -> publishRustState(effect)
            "publish_telemetry" -> publishRustTelemetry(effect)
            "publish_bms_state" -> publishRustBmsState(effect)
            "publish_bms_observation" -> publishRustBmsObservation(effect)
            "publish_session_marker" -> {
                applyRustSessionMarker(effect)
                events.publish("rust_session_commit")
            }
            "session_result" -> {
                rustSessionResult = effect.objectOrNull("value")
                    ?: effect.arrayOrNull("value")?.let { json("rows" to it) }
            }
            "store_configuration_cache" -> {
                val identity = cacheIdentity(effect.objectOrNull("identity"))
                val bytes = effect.intList("bytes")?.map(Int::toByte)?.toByteArray()
                if (identity != null && bytes != null) productStore?.storeRefloatSchema(bytes, identity)
            }
            "remove_configuration_cache" -> cacheIdentity(effect.objectOrNull("identity"))?.let {
                productStore?.removeCachedRefloatSchema(it)
            }
            "mutation" -> finishMutation(effect.optString("value"))
            "failure" -> finishRuntimeFailure(effect.optString("kind"))
            "diagnostic" -> events.publish("rust_runtime_diagnostic")
            else -> throw runtimeFailure()
        }
    }

    private fun armRustTimer(effect: JSONObject, repeating: Boolean, deadline: Boolean) {
        val token = effect.long("token") ?: throw runtimeFailure()
        val interval = effect.long(if (effect.has("timeout_ms")) "timeout_ms" else "interval_ms")
            ?.takeIf { it > 0 } ?: throw runtimeFailure()
        if (repeating && rustTimerIntervals[token] == interval && rustTimers[token] != null) return
        val command = if (deadline) "deadline_fired" else "timer_fired"
        val action = {
            synchronized(this) {
                runCatching { performRustCommand(json(
                    "command" to command, "generation" to rustGeneration,
                    "token" to token, "now_ms" to clock.monotonicMillis(),
                )) }.onFailure { events.publish("rust_runtime_timer_failure") }
                if (deadline) rustDeadlines.remove(token)?.cancel()
            }
        }
        val timer = scheduler ?: SystemScheduler()
        val cancellation = if (repeating || deadline) timer.scheduleRepeating(interval, action)
            else timer.schedule(interval, action)
        if (deadline) {
            rustDeadlines.remove(token)?.cancel()
            rustDeadlines[token] = cancellation
        } else {
            rustTimers.remove(token)?.cancel()
            rustTimers[token] = cancellation
            rustTimerIntervals[token] = interval
        }
    }

    private fun cancelRustTimers() {
        rustDeadlines.values.forEach(CancellationPort::cancel)
        rustDeadlines.clear()
        rustTimers.values.forEach(CancellationPort::cancel)
        rustTimers.clear()
        rustTimerIntervals.clear()
    }

    private fun prepareRustActivation(effect: JSONObject) {
        val generation = effect.long("generation") ?: throw runtimeFailure()
        val route = controllerRoute(effect.objectOrNull("selected_route")) ?: throw runtimeFailure()
        val firmware = firmware(effect.objectOrNull("selected_identity"), route) ?: throw runtimeFailure()
        val packageVersion = effect.string("refloat_version") ?: throw runtimeFailure()
        rustGeneration = generation
        rustControllerRoute = route
        rustFirmware = firmware
        rustRefloatPackageVersion = packageVersion
        if (trackingMode == TrackingMode.SAVED) {
            activeDeviceId?.let { deviceId ->
                currentSavedDevice = productStore?.ensureSavedDevice(
                    deviceId, activeDeviceName, stableBoardIdentity(firmware.controllerUuid), clock.now(),
                )
            }
        }
        var sessionStarted = false
        if (trackingMode == TrackingMode.SAVED && logStore.active() == null) {
            activeDeviceId?.let { deviceId ->
                events.publish(logStore.create(deviceId, activeDeviceName, clock.now()))
                sessionStarted = true
                sessionControllerRoute = route
                sessionControllerUuid = firmware.controllerUuid
            }
        }
        val activePath = if (trackingMode == TrackingMode.SAVED) logStore.activeSessionPath() else null
        if (!rustSessionAttached && activePath != null) {
            // Android framework SQLite and bundled Rust SQLite must not retain
            // concurrent writer handles to the same WAL database. Creation is
            // native catalogue work; transfer the session writer before Rust
            // validates and opens the canonical file.
            logStore.releaseActiveSessionWriter()
            performRustCommand(json(
                "command" to "attach_session", "path" to activePath, "mode" to "open",
                "expected_riding_persistence_ms" to samplingSettings.ridingPersistIntervalMs,
                "expected_idle_persistence_ms" to samplingSettings.idlePersistIntervalMs,
                "committed_at_epoch_ms" to clock.now().toEpochMilli(),
            ))
            rustSessionAttached = true
        }
        var cached: Any? = null
        cacheIdentity(effect.objectOrNull("configuration_cache_identity"))?.let { identity ->
            cached = productStore?.cachedRefloatSchema(identity)?.unsigned()
        }
        performRustCommand(json(
            "command" to "activate", "generation" to generation,
            "now_ms" to clock.monotonicMillis(), "session_started" to sessionStarted,
            "cached_configuration_schema" to cached,
        ))
    }

    private fun storeRustRoutingHint(effect: JSONObject) {
        if (trackingMode != TrackingMode.SAVED) return
        val savedId = currentSavedDevice?.savedDeviceId ?: return
        val route = controllerRoute(effect.objectOrNull("route")) as? ControllerRoute.Can ?: return
        val uuid = effect.objectOrNull("identity")?.intList("controller_uuid")
            ?.map(Int::toByte)?.toByteArray() ?: return
        productStore?.storeControllerRoutingHint(
            ControllerRoutingHint(savedId, route.canId, uuid), clock.now(),
        )
    }

    private fun publishRustState(effect: JSONObject) {
        rustGeneration = effect.long("generation") ?: throw runtimeFailure()
        val phase = effect.string("phase") ?: throw runtimeFailure()
        rustControllerRoute = controllerRoute(effect.objectOrNull("controller_route"))
        rustFirmware = firmware(effect.objectOrNull("selected_firmware"), rustControllerRoute)
        rustControllerCandidates = effect.array("controller_candidates").objects().mapNotNull { item ->
            ControllerCandidate(
                item.int("can_id") ?: return@mapNotNull null,
                item.string("hardware_name"), item.string("firmware_name"),
                item.string("vesc_version") ?: return@mapNotNull null,
                item.boolean("vesc_supported") ?: return@mapNotNull null,
            )
        }
        rustConnectionIssue = effect.objectOrNull("connection_issue")?.let { issue ->
            val code = connectionIssueCode(issue.string("code")) ?: return@let null
            ConnectionIssue(code, issue.string("message") ?: return@let null)
        }
        currentLimits = effect.objectOrNull("current_limits")?.let { limits ->
            ControllerCurrentLimits(
                limits.double("motor_current_max_a") ?: 0.0,
                limits.double("motor_current_min_a") ?: 0.0,
                limits.double("battery_current_max_a") ?: 0.0,
                limits.double("battery_current_min_a") ?: 0.0,
                limits.int("battery_series_cells"),
            )
        }
        rustControls = controls(effect.objectOrNull("controls"))
        effect.objectOrNull("configuration")?.let(::updateRustConfiguration)
        effect.objectOrNull("controller_actions")?.let { actions ->
            rustControllerActions = ControllerActions(
                actions.optBoolean("shutdown_available"),
                if (actions.optBoolean("shutdown_pending")) ControllerPendingAction.SHUTDOWN else null,
            )
        }
        deviceKind = if (effect.optBoolean("standalone_bms")) DeviceKind.BMS
            else if (rustFirmware == null) null else DeviceKind.BOARD
        transition(when (phase) {
            "identifying", "awaitingactivation" -> ConnectionState.IDENTIFYING
            "selectingcontroller" -> ConnectionState.SELECTING_CONTROLLER
            "ready" -> ConnectionState.READY
            "failed" -> ConnectionState.FAILED
            else -> ConnectionState.DISCONNECTED
        })
    }

    private fun publishRustTelemetry(effect: JSONObject) {
        val ride = rideState(effect.string("ride_state")) ?: throw runtimeFailure()
        val extensions = effect.array("extensions").objects().mapNotNull { item ->
            val name = item.string("name") ?: return@mapNotNull null
            name to (item.double("value") ?: return@mapNotNull null)
        }.toMap()
        val sample = TelemetrySample(
            sequence = effect.requireLong("sequence"),
            observedAt = Instant.ofEpochMilli(effect.requireLong("observed_at_epoch_ms")),
            elapsedMs = effect.requireLong("elapsed_ms"),
            rideState = ride,
            slowObservedAt = effect.long("slow_observed_at_epoch_ms")?.let(Instant::ofEpochMilli),
            packageState = wireEnum<PackageState>(effect.string("package_state")),
            packageMode = wireEnum<PackageMode>(normalized(effect.string("package_mode"))),
            footpadState = wireEnum<FootpadState>(effect.string("footpad_state")),
            stopCondition = wireEnum<StopCondition>(normalized(effect.string("stop_condition"))),
            setpointAdjustment = wireEnum<SetpointAdjustment>(normalized(effect.string("setpoint_adjustment"))),
            wheelslip = effect.boolean("wheelslip"), darkride = effect.boolean("darkride"),
            charging = effect.boolean("charging"),
            alertReason = wireEnum<AlertReason>(normalized(effect.string("alert_reason"))),
            firmwareFault = wireEnum<FirmwareFault>(normalized(effect.string("firmware_fault"))),
            packVoltageV = effect.double("pack_voltage_v"), dutyCycle = effect.double("duty_cycle"),
            controllerTempC = effect.double("controller_temp_c"), motorTempC = effect.double("motor_temp_c"),
            motorCurrentA = effect.double("motor_current_a"), batteryCurrentA = effect.double("battery_current_a"),
            erpm = effect.double("erpm"), speedKph = effect.double("speed_kph"),
            pitchDeg = effect.double("pitch_deg"), balancePitchDeg = effect.double("balance_pitch_deg"),
            rollDeg = effect.double("roll_deg"),
            directionalMotorCurrentA = effect.double("directional_motor_current_a"),
            filteredMotorCurrentA = effect.double("filtered_motor_current_a"),
            requestedBalanceCurrentA = effect.double("requested_balance_current_a"),
            boosterCurrentA = effect.double("booster_current_a"),
            fieldWeakeningCurrentA = effect.double("field_weakening_current_a"),
            footpadAdc1V = effect.double("footpad_adc1_v"), footpadAdc2V = effect.double("footpad_adc2_v"),
            remoteInput = effect.double("remote_input"), setpointDeg = effect.double("setpoint_deg"),
            atrSetpointDeg = effect.double("atr_setpoint_deg"),
            brakeTiltSetpointDeg = effect.double("brake_tilt_setpoint_deg"),
            torqueTiltSetpointDeg = effect.double("torque_tilt_setpoint_deg"),
            turnTiltSetpointDeg = effect.double("turn_tilt_setpoint_deg"),
            remoteSetpointDeg = effect.double("remote_setpoint_deg"),
            atrAccelerationDifferenceErpmPerSecond = effect.double("atr_acceleration_difference_erpm_per_second"),
            atrSpeedBoost = effect.double("atr_speed_boost"), distanceM = effect.double("distance_m"),
            odometerM = effect.double("odometer_m"), ampHoursDrawnAh = effect.double("amp_hours_drawn_ah"),
            ampHoursChargedAh = effect.double("amp_hours_charged_ah"),
            wattHoursDrawnWh = effect.double("watt_hours_drawn_wh"),
            wattHoursChargedWh = effect.double("watt_hours_charged_wh"),
            batteryLevel = effect.double("battery_level"),
            chargingCurrentA = effect.double("charging_current_a"),
            chargingVoltageV = effect.double("charging_voltage_v"), extension = extensions,
        )
        latestPersistedSample = sample
        updateLocationCapture(ride)
        events.publish(sample)
    }

    private fun publishRustBmsState(effect: JSONObject) {
        val status = BmsStatus.entries.firstOrNull { it.wireValue == effect.string("status") }
            ?: throw runtimeFailure()
        val candidates = effect.array("candidates").objects().mapNotNull { item ->
            val route = item.int("can_id")?.let(BmsConnection::Can) ?: BmsConnection.Direct
            BmsCandidate(route, item.string("hardware_name"), item.string("version") ?: return@mapNotNull null)
        }
        rustBmsState = BmsState(status, bmsConnection(effect.objectOrNull("route")), candidates,
            rustBmsState.latestSnapshot)
        events.publish(rustBmsState)
    }

    private fun publishRustBmsObservation(effect: JSONObject) {
        val snapshot = BmsSnapshot(
            Instant.ofEpochMilli(effect.requireLong("observed_at_epoch_ms")),
            effect.requireDouble("total_voltage_v"), effect.requireDouble("charge_voltage_v"),
            effect.requireDouble("input_current_a"), effect.requireDouble("input_ic_current_a"),
            effect.requireDouble("session_amp_hours_ah"), effect.requireDouble("session_watt_hours_wh"),
            effect.doubleList("cell_voltages_v") ?: throw runtimeFailure(),
            effect.booleanList("balancing") ?: throw runtimeFailure(),
            effect.doubleList("temperatures_c") ?: throw runtimeFailure(),
            effect.requireDouble("ic_temperature_c"), effect.requireDouble("humidity_sensor_temperature_c"),
            effect.requireDouble("humidity_percent"), effect.requireDouble("highest_cell_temperature_c"),
            effect.requireDouble("state_of_charge"), effect.requireDouble("state_of_health"),
            effect.int("reported_can_id") ?: throw runtimeFailure(),
            effect.requireDouble("lifetime_charge_amp_hours_ah"),
            effect.requireDouble("lifetime_charge_watt_hours_wh"),
            effect.requireDouble("lifetime_discharge_amp_hours_ah"),
            effect.requireDouble("lifetime_discharge_watt_hours_wh"), effect.requireDouble("pressure_pa"),
            effect.int("data_version"), effect.string("status"),
        )
        rustBmsState = BmsState(
            BmsStatus.ATTACHED, bmsConnection(effect.objectOrNull("route")),
            rustBmsState.candidates, snapshot,
        )
        events.publish(snapshot)
    }

    private fun applyRustSessionMarker(effect: JSONObject) {
        val active = logStore.active() ?: throw runtimeFailure()
        val snapshot = SessionPublishedSnapshotRecord(
            effect.requireLong("history_revision"), effect.requireLong("committed_sample_sequence"),
            effect.requireLong("committed_event_sequence"), effect.requireLong("committed_elapsed_ms"),
            effect.requireLong("recorded_sample_count"), effect.requireLong("stored_sample_count"),
            effect.requireLong("committed_location_sequence"),
            effect.requireLong("committed_location_elapsed_ms"),
            effect.requireLong("recorded_location_count"), effect.requireLong("stored_location_count"),
            effect.string("last_location_status") ?: throw runtimeFailure(),
        )
        sessions().applySessionCommitMarker(
            active.sessionId.value,
            SessionCommitMarkerRecord(
                snapshot, effect.requireLong("committed_event_count"),
                Instant.ofEpochMilli(effect.requireLong("committed_at_epoch_ms")),
                effect.requireLong("riding_elapsed_ms"), effect.double("distance_m"),
                effect.double("maximum_speed_kph"), effect.long("maximum_speed_source_sequence"),
                effect.double("maximum_duty_cycle"), effect.long("maximum_duty_source_sequence"),
                effect.double("last_speed_kph"), effect.string("last_ride_state"),
            ),
        )
    }

    private fun updateRustConfiguration(value: JSONObject) {
        val availability = wireEnum<RefloatConfigurationAvailability>(value.string("availability"))
            ?: throw runtimeFailure()
        val pending = when (normalized(value.string("pending"))) {
            "schema" -> RefloatConfigurationOperation.SCHEMA
            "readcurrent" -> RefloatConfigurationOperation.READ_CURRENT
            "readdefault" -> RefloatConfigurationOperation.READ_DEFAULT
            "write" -> RefloatConfigurationOperation.WRITE
            null -> null
            else -> throw runtimeFailure()
        }
        val identity = value.objectOrNull("identity")
        rustConfigurationSchema = configurationSchema(value.objectOrNull("schema"))
        rustConfigurationCurrent = configurationValues(
            value.objectOrNull("current_values"), value.long("current_revision"), "current",
        )
        rustConfigurationDefaults = configurationValues(
            value.objectOrNull("default_values"), value.long("default_revision"), "default",
        )
        rustConfigurationSummary = RefloatConfigurationSummary(
            availability, value.boolean("writable") ?: throw runtimeFailure(),
            identity?.string("package_version"), identity?.string("fingerprint"),
            identity?.long("wire_signature"), rustConfigurationCurrent != null,
            rustConfigurationDefaults != null, pending,
        )
        events.publish(rustConfigurationSummary)
        if (pending != null && refloatConfigurationWaiter != null) {
            refloatConfigurationObservedPending = true
        } else if (pending == null && refloatConfigurationObservedPending) {
            finishRefloatConfigurationRequest(Result.success(Unit))
        }
    }

    private fun configurationSchema(value: JSONObject?): RefloatConfigurationSchema? {
        value ?: return null
        val fields = value.array("fields").objects().map { field ->
            val fieldType = when (normalized(field.string("field_type"))) {
                "decimal" -> RefloatConfigurationFieldType.DOUBLE
                "integer" -> RefloatConfigurationFieldType.INTEGER
                "string" -> RefloatConfigurationFieldType.STRING
                "enumeration" -> RefloatConfigurationFieldType.ENUMERATION
                "boolean" -> RefloatConfigurationFieldType.BOOLEAN
                "bitfield" -> RefloatConfigurationFieldType.BITFIELD
                else -> throw runtimeFailure()
            }
            val wireType = when (normalized(field.string("wire_type"))) {
                "uint8" -> RefloatConfigurationWireType.UINT8
                "int8" -> RefloatConfigurationWireType.INT8
                "uint16" -> RefloatConfigurationWireType.UINT16
                "int16" -> RefloatConfigurationWireType.INT16
                "uint32" -> RefloatConfigurationWireType.UINT32
                "int32" -> RefloatConfigurationWireType.INT32
                "decimal16" -> RefloatConfigurationWireType.DOUBLE16
                "decimal32" -> RefloatConfigurationWireType.DOUBLE32
                "decimal32auto" -> RefloatConfigurationWireType.DOUBLE32_AUTO
                null -> null
                else -> throw runtimeFailure()
            }
            RefloatConfigurationField(
                field.string("name") ?: throw runtimeFailure(),
                field.string("long_name") ?: throw runtimeFailure(),
                fieldType, wireType,
                configurationValue(field.objectOrNull("default_value") ?: throw runtimeFailure()),
                field.double("minimum"), field.double("maximum"), field.double("step"),
                field.int("precision"), field.double("scale"),
                field.string("suffix") ?: throw runtimeFailure(),
                field.array("choices").strings(),
                field.string("description") ?: throw runtimeFailure(),
            )
        }
        val groups = value.array("groups").objects().map { group ->
            RefloatConfigurationGroup(
                group.string("name") ?: throw runtimeFailure(),
                group.array("subgroups").objects().map { subgroup ->
                    RefloatConfigurationSubgroup(
                        subgroup.string("name") ?: throw runtimeFailure(),
                        subgroup.array("fields").strings(),
                    )
                },
            )
        }
        return RefloatConfigurationSchema(
            value.string("fingerprint") ?: throw runtimeFailure(),
            value.long("wire_signature") ?: throw runtimeFailure(), fields, groups,
        )
    }

    private fun configurationValues(
        value: JSONObject?, revision: Long?, source: String,
    ): RefloatConfigurationValues? {
        value ?: return null
        val actualRevision = revision ?: throw runtimeFailure()
        val values = value.keys().asSequence().associateWith { name ->
            configurationValue(value.optJSONObject(name) ?: throw runtimeFailure())
        }
        return RefloatConfigurationValues(actualRevision, source, values)
    }

    private fun configurationValue(value: JSONObject): RefloatConfigurationValue = when (value.string("type")) {
        "double" -> RefloatConfigurationValue.Decimal(value.double("value") ?: throw runtimeFailure())
        "integer" -> RefloatConfigurationValue.Integer(value.long("value") ?: throw runtimeFailure())
        "boolean" -> RefloatConfigurationValue.BooleanValue(value.boolean("value") ?: throw runtimeFailure())
        "string" -> RefloatConfigurationValue.StringValue(value.string("value") ?: throw runtimeFailure())
        else -> throw runtimeFailure()
    }

    private fun configurationValuesJson(values: Map<String, RefloatConfigurationValue>): JSONObject =
        JSONObject().also { result -> values.toSortedMap().forEach { (name, value) ->
            result.put(name, when (value) {
                is RefloatConfigurationValue.Decimal -> json("type" to "double", "value" to value.value)
                is RefloatConfigurationValue.Integer -> json("type" to "integer", "value" to value.value)
                is RefloatConfigurationValue.BooleanValue -> json("type" to "boolean", "value" to value.value)
                is RefloatConfigurationValue.StringValue -> json("type" to "string", "value" to value.value)
            })
        } }

    private fun configurationValuesString(values: Map<String, RefloatConfigurationValue>) =
        configurationValuesJson(values).toString()

    private fun configurationValues(value: JSONObject): Map<String, RefloatConfigurationValue> =
        value.keys().asSequence().associateWith { name ->
            configurationValue(value.optJSONObject(name) ?: throw ProductStoreException.InvalidRefloatConfiguration)
        }

    private fun finishMutation(value: String) {
        when {
            value.contains("Failed") -> {
                refloatWaiter?.finish(Result.failure(RefloatMutationException(unavailable = true)))
                refloatWaiter = null
            }
            listOf("Idempotent", "Confirmed", "Released").any(value::contains) -> {
                rustControls?.let { refloatWaiter?.finish(Result.success(it)) }
                refloatWaiter = null
            }
        }
    }

    private fun finishRuntimeFailure(failure: String) {
        events.publish("rust_runtime_failure")
        if (failure.startsWith("Mutation")) {
            refloatWaiter?.finish(Result.failure(RefloatMutationException(unavailable = true)))
            refloatWaiter = null
        }
        if (failure.startsWith("Configuration") || failure.contains("ConfigurationWrite") ||
            failure.contains("TuneApply")
        ) {
            finishRefloatConfigurationRequest(Result.failure(configurationFailure(failure)))
        }
    }

    private fun finishRefloatConfigurationRequest(result: Result<Unit>) {
        val waiter = refloatConfigurationWaiter
        refloatConfigurationWaiter = null
        refloatConfigurationObservedPending = false
        waiter?.finish(result)
    }

    private fun configurationFailure(value: String): Throwable = RefloatConfigurationServiceException(when {
        value.contains("Conflict") -> RefloatConfigurationServiceError.CONFLICT
        value.contains("StaleResponse") -> RefloatConfigurationServiceError.STALE_RESPONSE
        value.contains("UnauditedSchema") -> RefloatConfigurationServiceError.UNAUDITED_SCHEMA
        value.contains("MalformedResponse") || value.contains("Codec") ->
            RefloatConfigurationServiceError.MALFORMED_RESPONSE
        value.contains("InvalidState") -> RefloatConfigurationServiceError.INVALID_STATE
        else -> RefloatConfigurationServiceError.UNAVAILABLE
    })

    private fun cacheIdentity(value: JSONObject?): RefloatSchemaCacheIdentity? = value?.let {
        RefloatSchemaCacheIdentity(
            it.string("firmware_version") ?: return null,
            it.string("hardware_name") ?: return null,
            it.string("package_version") ?: return null,
            it.int("custom_index") ?: return null,
            it.long("hardware_configuration_crc") ?: return null,
        )
    }

    private fun firmware(value: JSONObject?, route: ControllerRoute?): FirmwareRecord? = value?.let {
        FirmwareRecord(
            it.int("major") ?: return null, it.int("minor") ?: return null,
            it.string("hardware_name"), it.string("firmware_name"),
            it.int("hardware_type") ?: return null, routeCanId(route),
            it.intList("controller_uuid")?.map(Int::toByte)?.toByteArray(),
            it.int("test_version") ?: return null,
            it.int("custom_configuration_count") ?: return null,
            it.long("hardware_configuration_crc"),
        )
    }

    private fun controllerRoute(value: JSONObject?): ControllerRoute? = when (value?.string("kind")) {
        "direct" -> ControllerRoute.Direct
        "can" -> value.int("can_id")?.let(ControllerRoute::Can)
        else -> null
    }

    private fun routeCanId(route: ControllerRoute?): Int? = (route as? ControllerRoute.Can)?.canId

    private fun bmsConnection(value: JSONObject?): BmsConnection? = when (val route = controllerRoute(value)) {
        ControllerRoute.Direct -> BmsConnection.Direct
        is ControllerRoute.Can -> BmsConnection.Can(route.canId)
        null -> null
    }

    private fun controls(value: JSONObject?): RefloatControls? = value?.let {
        val status = wireEnum<RefloatControlStatus>(it.string("status")) ?: return null
        val pending = wireEnum<RefloatPendingAction>(normalized(it.string("pending_action")))
        RefloatControls(
            status, it.boolean("writable") ?: return null,
            it.boolean("package_enabled"), it.boolean("flywheel_enabled"),
            it.boolean("lighting_available") ?: return null,
            it.boolean("leds_enabled"), it.boolean("headlights_enabled"),
            it.boolean("fatal_error_clear_available") ?: return null,
            it.boolean("fatal_error_latched"), it.boolean("fatal_error_active"),
            it.boolean("fatal_error_clearable"), pending,
        )
    }

    private fun stableBoardIdentity(value: ByteArray?): String? = value?.takeIf { it.size == 12 }?.let {
        MessageDigest.getInstance("SHA-256").digest(it).joinToString("") { byte -> "%02x".format(byte) }
    }

    private fun preferredCanId(valuesJson: String, key: String): Int? = runCatching {
        JSONObject(valuesJson).takeIf { !it.isNull(key) }?.getInt(key)?.takeIf { it in 0..253 }
    }.getOrNull()

    private fun permissionAllowsUse(value: BluetoothPermissionState) =
        value in setOf(BluetoothPermissionState.GRANTED, BluetoothPermissionState.NOT_REQUIRED)

    private fun permissionIsTerminal(value: BluetoothPermissionState) =
        value in setOf(BluetoothPermissionState.DENIED, BluetoothPermissionState.RESTRICTED)

    private fun connectionIssueCode(value: String?): ConnectionIssueCode? =
        ConnectionIssueCode.entries.firstOrNull { normalized(it.name) == normalized(value) }

    private fun rideState(value: String?): RideState? = wireEnum<RideState>(value)

    private inline fun <reified T : Enum<T>> wireEnum(value: String?): T? =
        enumValues<T>().firstOrNull { normalized(it.name) == normalized(value) }

    private fun normalized(value: String?): String? = value?.replace("_", "")?.lowercase()

    private fun runtimeFailure() = RustApplicationRuntimeException(RustApplicationRuntimeError.INTERNAL_FAILURE)

    companion object {
        private const val DISCOVERY_EXPIRY_MS = 15_000L
        private const val MAX_RECONNECT_BACKOFF_MS = 30_000L
    }
}

private fun json(vararg values: Pair<String, Any?>): JSONObject = JSONObject().also { objectValue ->
    values.forEach { (name, value) -> objectValue.put(name, jsonValue(value)) }
}

private fun jsonValue(value: Any?): Any = when (value) {
    null -> JSONObject.NULL
    is ByteArray -> JSONArray(value.unsigned())
    is Iterable<*> -> JSONArray().also { array -> value.forEach { array.put(jsonValue(it)) } }
    else -> value
}

private fun ByteArray.unsigned() = map { it.toInt() and 0xff }
private fun JSONObject.string(name: String): String? = if (isNull(name)) null else opt(name) as? String
private fun JSONObject.long(name: String): Long? = if (isNull(name) || !has(name)) null else runCatching {
    val value = get(name)
    when (value) {
        is Byte, is Short, is Int, is Long -> (value as Number).toLong()
        is Number -> value.toDouble().takeIf { it.isFinite() && it % 1.0 == 0.0 }?.toLong()
        else -> null
    }
}.getOrNull()
private fun JSONObject.int(name: String): Int? = long(name)?.takeIf { it in Int.MIN_VALUE..Int.MAX_VALUE }?.toInt()
private fun JSONObject.double(name: String): Double? = if (isNull(name) || !has(name)) null else runCatching {
    (get(name) as? Number)?.toDouble()?.takeIf(Double::isFinite)
}.getOrNull()
private fun JSONObject.boolean(name: String): Boolean? = if (isNull(name) || !has(name)) null else
    (opt(name) as? Boolean)
private fun JSONObject.objectOrNull(name: String): JSONObject? = if (isNull(name)) null else optJSONObject(name)
private fun JSONObject.arrayOrNull(name: String): JSONArray? = if (isNull(name)) null else optJSONArray(name)
private fun JSONObject.array(name: String): JSONArray = arrayOrNull(name) ?: JSONArray()
private fun JSONObject.requireLong(name: String) = long(name) ?: error("Missing integer $name")
private fun JSONObject.requireDouble(name: String) = double(name) ?: error("Missing number $name")
private fun JSONObject.longList(name: String): List<Long>? = arrayOrNull(name)?.let { array ->
    (0 until array.length()).map { index ->
        json("value" to array.opt(index)).long("value") ?: return null
    }
}
private fun JSONObject.intList(name: String): List<Int>? = longList(name)?.map { value ->
    value.takeIf { it in Int.MIN_VALUE..Int.MAX_VALUE }?.toInt() ?: return null
}
private fun JSONObject.doubleList(name: String): List<Double>? = arrayOrNull(name)?.let { array ->
    (0 until array.length()).map { index ->
        json("value" to array.opt(index)).double("value") ?: return null
    }
}
private fun JSONObject.optionalDoubleList(name: String): List<Double?>? = arrayOrNull(name)?.let { array ->
    (0 until array.length()).map { index ->
        if (array.isNull(index)) null else json("value" to array.opt(index)).double("value") ?: return null
    }
}
private fun JSONObject.booleanList(name: String): List<Boolean>? = arrayOrNull(name)?.let { array ->
    (0 until array.length()).map { index -> array.opt(index) as? Boolean ?: return null }
}
private fun JSONArray.objects(): List<JSONObject> =
    (0 until length()).mapNotNull(::optJSONObject)
private fun JSONArray.strings(): List<String> =
    (0 until length()).mapNotNull { index -> opt(index) as? String }

private class RefloatMutationWaiter {
    private val latch = CountDownLatch(1)
    private var result: Result<RefloatControls>? = null

    @Synchronized
    fun finish(value: Result<RefloatControls>) {
        if (result != null) return
        result = value
        latch.countDown()
    }

    fun await(): RefloatControls {
        if (!latch.await(3_250, TimeUnit.MILLISECONDS)) throw RefloatMutationException(unavailable = true)
        return requireNotNull(synchronized(this) { result }).getOrThrow()
    }
}

private class RefloatConfigurationRequestWaiter {
    private val latch = CountDownLatch(1)
    private var result: Result<Unit>? = null

    @Synchronized
    fun finish(value: Result<Unit>) {
        if (result != null) return
        result = value
        latch.countDown()
    }

    fun await() {
        if (!latch.await(3_750, TimeUnit.MILLISECONDS)) {
            throw RefloatConfigurationServiceException(RefloatConfigurationServiceError.UNAVAILABLE)
        }
        requireNotNull(synchronized(this) { result }).getOrThrow()
    }
}
