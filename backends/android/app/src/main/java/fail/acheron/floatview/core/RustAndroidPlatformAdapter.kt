package fail.acheron.floatview.core

import fail.acheron.floatview.backend.LocationProviderAuthorization
import fail.acheron.floatview.backend.LocationProviderBackgroundCapture
import fail.acheron.floatview.backend.LocationProviderObservation
import fail.acheron.floatview.backend.LocationProviderPort
import java.io.File
import java.net.URI
import java.nio.ByteBuffer
import java.nio.ByteOrder

internal class RustAndroidPlatformAdapter(
    private val runtime: CorePlatformRuntime,
    private val bluetooth: RustBluetoothPlatformAdapter,
    private val location: LocationProviderPort,
    private val host: AndroidPlatformHost,
) {
    private val lock = Any()
    private val pendingPresentations = mutableMapOf<Long, NativeHostCommand>()
    private var locationActive = false
    private var stopped = false

    fun dispatch(command: NativeHostCommand) {
        if (bluetooth.dispatch(command)) return
        val hostCommand =
            try {
                val envelope = PlatformCommandEnvelope.decode(command)
                AndroidHostCommand.decode(envelope.command as PlatformCommand.Host)
            } catch (_: Exception) {
                host.diagnostic("bridge_command", "invalid_payload")
                runtime.complete(command, RESULT_FAILED)
                return
            }
        if (synchronized(lock) { stopped }) {
            runtime.complete(command, RESULT_CANCELLED)
            return
        }
        when (hostCommand) {
            is AndroidHostCommand.LocationStart -> startLocation(command, hostCommand.background)
            AndroidHostCommand.LocationStop -> {
                synchronized(lock) { locationActive = false }
                location.stopUpdates()
                runtime.complete(command)
            }
            is AndroidHostCommand.SetBackgroundLogging -> completeBoolean(
                command,
                host.setBackgroundLogging(hostCommand.enabled),
            )
            is AndroidHostCommand.ReadResource -> completeResult(command) {
                host.readResource(
                    hostCommand.namespace,
                    hostCommand.logicalName,
                    hostCommand.maximumBytes,
                )
            }
            is AndroidHostCommand.StageDocument -> {
                if (!beginPresentation(command)) return
                host.stageDocument(File(hostCommand.destination), hostCommand.maximumBytes) { error ->
                    finishPresentation(command, error)
                }
            }
            is AndroidHostCommand.ResolveStorageRoot -> completeResult(command) {
                host.storageRoot(hostCommand.kind).canonicalPath.encodeToByteArray()
            }
            is AndroidHostCommand.AvailableSpace -> completeResult(command) {
                longBytes(host.availableSpace(File(hostCommand.root)))
            }
            AndroidHostCommand.MonotonicClock -> completeResult(command) {
                longBytes(host.monotonicMilliseconds())
            }
            AndroidHostCommand.WallClock -> completeResult(command) {
                longBytes(host.wallMilliseconds())
            }
            is AndroidHostCommand.SecureRandom -> completeResult(command) {
                host.secureRandom(hostCommand.count)
            }
            is AndroidHostCommand.Diagnostic -> {
                host.diagnostic(hostCommand.stage, hostCommand.code)
                runtime.complete(command)
            }
            is AndroidHostCommand.PresentPermission -> presentPermission(command, hostCommand.permission)
            AndroidHostCommand.PresentSettings -> {
                location.openSettings()
                runtime.complete(command)
            }
            is AndroidHostCommand.PresentTheme -> completeBoolean(
                command,
                host.presentTheme(hostCommand.theme),
            )
            is AndroidHostCommand.PresentServiceState -> completeBoolean(
                command,
                host.presentServiceState(hostCommand.active),
            )
            is AndroidHostCommand.PresentExport -> {
                if (!beginPresentation(command)) return
                host.presentExport(File(hostCommand.stagedPath)) { error ->
                    finishPresentation(command, error)
                }
            }
            is AndroidHostCommand.RuntimeReady -> {
                val endpoint = runCatching { URI(hostCommand.endpoint) }.getOrNull()
                val valid =
                    endpoint?.scheme == "http" && endpoint.host in setOf("127.0.0.1", "localhost") &&
                        endpoint.port in 1..65535 && hostCommand.launchToken.isNotEmpty()
                completeBoolean(
                    command,
                    valid && host.runtimeReady(hostCommand.endpoint, hostCommand.launchToken),
                )
            }
        }
    }

    fun updateLifecycle(foreground: Boolean, backgroundEligible: Boolean) =
        bluetooth.submitHostEvent(PlatformEvent.Lifecycle(foreground, backgroundEligible))

    fun updateLoggingService(active: Boolean) =
        bluetooth.submitHostEvent(PlatformEvent.LoggingService(active))

    fun updateTheme(theme: PlatformTheme) =
        bluetooth.submitHostEvent(PlatformEvent.Theme(theme))

    fun refreshLocationAuthorization() = submitLocationAuthorization(location.state().authorization)

    fun shutdown() {
        val pending = synchronized(lock) {
            if (stopped) return
            stopped = true
            locationActive = false
            pendingPresentations.values.toList().also { pendingPresentations.clear() }
        }
        location.stopUpdates()
        host.cancelPendingPresentation()
        bluetooth.shutdown()
        pending.forEach { runtime.complete(it, RESULT_CANCELLED) }
    }

    private fun startLocation(command: NativeHostCommand, background: Boolean) {
        val state = location.state()
        submitLocationAuthorization(state.authorization)
        val authorized =
            state.authorization == LocationProviderAuthorization.AUTHORIZED ||
                state.authorization == LocationProviderAuthorization.FOREGROUND_ONLY
        if (!authorized) {
            runtime.complete(
                command,
                if (state.authorization == LocationProviderAuthorization.DENIED) {
                    RESULT_PERMISSION_DENIED
                } else {
                    RESULT_UNAVAILABLE
                },
            )
            return
        }
        if (background && state.backgroundCapture != LocationProviderBackgroundCapture.AVAILABLE) {
            runtime.complete(command, RESULT_PERMISSION_DENIED)
            return
        }
        synchronized(lock) { locationActive = true }
        location.startUpdates(::receiveLocation)
        runtime.complete(command)
    }

    private fun receiveLocation(value: LocationProviderObservation) {
        if (!synchronized(lock) { locationActive && !stopped }) return
        bluetooth.submitHostEvent(
            PlatformEvent.Location(value.latitude, value.longitude, value.accuracyM),
        )
    }

    private fun submitLocationAuthorization(value: LocationProviderAuthorization) {
        val mapped =
            when (value) {
                LocationProviderAuthorization.DENIED, LocationProviderAuthorization.RESTRICTED ->
                    LocationAuthorization.DENIED
                LocationProviderAuthorization.FOREGROUND_ONLY -> LocationAuthorization.FOREGROUND
                LocationProviderAuthorization.AUTHORIZED -> LocationAuthorization.BACKGROUND
                else -> LocationAuthorization.UNKNOWN
            }
        bluetooth.submitHostEvent(PlatformEvent.LocationAuthorizationChanged(mapped))
    }

    private fun presentPermission(command: NativeHostCommand, permission: PermissionKind) {
        val presented =
            when (permission) {
                PermissionKind.BLUETOOTH -> {
                    bluetooth.requestPermission()
                    true
                }
                PermissionKind.LOCATION -> {
                    location.requestPermission()
                    true
                }
                PermissionKind.BACKGROUND_LOCATION -> {
                    location.openSettings()
                    true
                }
                PermissionKind.NOTIFICATIONS -> host.presentNotificationsPermission()
            }
        completeBoolean(command, presented)
    }

    private fun beginPresentation(command: NativeHostCommand): Boolean = synchronized(lock) {
        val accepted =
            !stopped && pendingPresentations.size < MAX_PENDING_PRESENTATIONS &&
                pendingPresentations.putIfAbsent(command.operationId, command) == null
        if (!accepted) runtime.complete(command, RESULT_FAILED)
        accepted
    }

    private fun finishPresentation(command: NativeHostCommand, error: AndroidPlatformHostError?) {
        val accepted = synchronized(lock) {
            pendingPresentations.remove(command.operationId) != null && !stopped
        }
        if (accepted) runtime.complete(command, resultKind(error))
    }

    private fun completeResult(command: NativeHostCommand, operation: () -> ByteArray) {
        try {
            runtime.complete(command, payload = operation())
        } catch (_: Exception) {
            runtime.complete(command, RESULT_FAILED)
        }
    }

    private fun completeBoolean(command: NativeHostCommand, success: Boolean) =
        runtime.complete(command, if (success) RESULT_SUCCESS else RESULT_UNAVAILABLE)

    private fun resultKind(error: AndroidPlatformHostError?): Int =
        when (error) {
            null -> RESULT_SUCCESS
            AndroidPlatformHostError.UNAVAILABLE -> RESULT_UNAVAILABLE
            else -> RESULT_FAILED
        }

    private fun longBytes(value: Long): ByteArray =
        ByteBuffer.allocate(8).order(ByteOrder.BIG_ENDIAN).putLong(value).array()

    private companion object {
        const val RESULT_SUCCESS = 0
        const val RESULT_CANCELLED = 1
        const val RESULT_PERMISSION_DENIED = 2
        const val RESULT_UNAVAILABLE = 3
        const val RESULT_FAILED = 4
        const val MAX_PENDING_PRESENTATIONS = 4
    }
}
