package fail.acheron.floatview.core

import fail.acheron.floatview.backend.LocationProviderAuthorization
import fail.acheron.floatview.backend.LocationProviderBackgroundCapture
import fail.acheron.floatview.backend.LocationProviderObservation
import fail.acheron.floatview.backend.LocationProviderPort
import java.net.URI

internal class FloatViewRuntimeBridgeAdapter(
    private val runtime: CorePlatformRuntime,
    private val bluetooth: RustBluetoothPlatformAdapter,
    private val location: LocationProviderPort,
    private val host: FloatViewRuntimeHostProtocol,
) {
    private val lock = Any()
    private var locationActive = false
    private var stopped = false

    init {
        location.setStateHandler { state -> submitLocationAuthorization(state.authorization) }
    }

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
            is AndroidHostCommand.PresentPermission -> presentPermission(command, hostCommand.permission)
            AndroidHostCommand.PresentSettings -> {
                location.openSettings()
                runtime.complete(command)
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
            is AndroidHostCommand.PresentImport -> host.presentImport(
                hostCommand.stagingPath,
                hostCommand.allowedContentTypes,
                hostCommand.maximumBytes,
            ) { result -> completeDocument(command, result) }
            is AndroidHostCommand.PresentExport -> host.presentExport(
                hostCommand.stagingPath,
                hostCommand.displayName,
                hostCommand.contentType,
            ) { result -> completeDocument(command, result) }
        }
    }

    fun updateLifecycle(foreground: Boolean, backgroundEligible: Boolean) =
        bluetooth.submitHostEvent(PlatformEvent.Lifecycle(foreground, backgroundEligible))

    fun shutdown() {
        synchronized(lock) {
            if (stopped) return
            stopped = true
            locationActive = false
        }
        location.stopUpdates()
        bluetooth.shutdown()
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

    private fun completeBoolean(command: NativeHostCommand, success: Boolean) =
        runtime.complete(command, if (success) RESULT_SUCCESS else RESULT_UNAVAILABLE)

    private fun completeDocument(command: NativeHostCommand, result: DocumentResult) =
        runtime.complete(
            command,
            when (result) {
                DocumentResult.SUCCESS -> RESULT_SUCCESS
                DocumentResult.CANCELLED -> RESULT_CANCELLED
                DocumentResult.UNAVAILABLE -> RESULT_UNAVAILABLE
                DocumentResult.FAILED -> RESULT_FAILED
            },
        )

    private companion object {
        const val RESULT_SUCCESS = 0
        const val RESULT_CANCELLED = 1
        const val RESULT_PERMISSION_DENIED = 2
        const val RESULT_UNAVAILABLE = 3
        const val RESULT_FAILED = 4
    }
}
