package fail.acheron.floatview

import android.content.res.AssetManager
import android.util.Base64
import fail.acheron.floatview.backend.ActiveConnectionSnapshot
import fail.acheron.floatview.backend.ApplicationService
import fail.acheron.floatview.backend.BluetoothState
import fail.acheron.floatview.backend.BoardProfileRecord
import fail.acheron.floatview.backend.ConnectionState
import fail.acheron.floatview.backend.DeviceId
import fail.acheron.floatview.backend.DiscoveredDevice
import fail.acheron.floatview.backend.DiscoverySnapshot
import fail.acheron.floatview.backend.EventHub
import fail.acheron.floatview.backend.ProductStoreException
import fail.acheron.floatview.backend.SessionExportRecord
import fail.acheron.floatview.backend.SessionGroupRecord
import fail.acheron.floatview.backend.SessionListQuery
import fail.acheron.floatview.backend.SessionRecord
import fail.acheron.floatview.backend.SessionSeriesPoint
import fail.acheron.floatview.backend.SessionStoreException
import fail.acheron.floatview.backend.SessionSummaryRecord
import fail.acheron.floatview.backend.SettingsRecord
import fail.acheron.floatview.backend.TelemetrySample
import fail.acheron.floatview.backend.VescProtocolMessage
import fail.acheron.floatview.protocol.refloat.RefloatCompatibility
import fail.acheron.floatview.protocol.vesc.ControllerRoute
import fail.acheron.floatview.protocol.vesc.VescCompatibility
import io.ktor.http.ContentType
import io.ktor.server.application.install
import io.ktor.server.cio.CIO
import io.ktor.server.engine.EmbeddedServer
import io.ktor.server.engine.embeddedServer
import io.ktor.server.request.header
import io.ktor.server.response.respondBytes
import io.ktor.server.response.respondText
import io.ktor.server.routing.get
import io.ktor.server.routing.routing
import io.ktor.server.websocket.WebSockets
import io.ktor.server.websocket.webSocket
import io.ktor.websocket.CloseReason
import io.ktor.websocket.Frame
import io.ktor.websocket.close
import io.ktor.websocket.readText
import io.ktor.websocket.send
import java.security.SecureRandom
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.selects.select
import org.json.JSONArray
import org.json.JSONObject

internal const val ANDROID_BACKEND_PORT = 11012

class LoopbackServer(
    private val assets: AssetManager,
    private val applicationService: ApplicationService,
    private val exportPresenter: ((SessionExportRecord) -> Boolean)?,
    private val events: EventHub,
    private val requestedPort: Int = 0,
) {
    private val token = ByteArray(32).also(SecureRandom()::nextBytes)
        .let { Base64.encodeToString(it, Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING) }
    private val revision = AtomicInteger()
    private var server: EmbeddedServer<*, *>? = null
    private var scanStopJob: Job? = null
    private var boundPort = 0
    val port: Int get() = boundPort
    val origin: String get() = "http://127.0.0.1:$port"

    fun start() {
        if (server != null) return
        val html = assets.open("index.html").use { it.readBytes() }
        val started = embeddedServer(CIO, host = "127.0.0.1", port = requestedPort) {
            install(WebSockets) { maxFrameSize = MAX_MESSAGE_BYTES.toLong() }
            routing {
                get("/") {
                    call.respondBytes(html, ContentType.Text.Html)
                }
                get("/api/bootstrap") {
                    call.respondText(
                        JSONObject()
                            .put("webSocketUrl", "ws://127.0.0.1:$port/api/ws")
                            .put("sessionToken", token)
                            .toString(),
                        ContentType.Application.Json,
                    )
                }
                webSocket("/api/ws") {
                    if (
                        call.request.queryParameters["token"] != token ||
                        call.request.header("Origin") != origin
                    ) {
                        close(CloseReason(CloseReason.Codes.VIOLATED_POLICY, "Forbidden"))
                        return@webSocket
                    }
                    val responses = Channel<String>(capacity = 1)
                    val stateUpdates = Channel<String>(capacity = Channel.CONFLATED)
                    val sender = launch {
                        var running = true
                        while (running) {
                            select {
                                responses.onReceiveCatching { result ->
                                    result.getOrNull()?.let { send(it) } ?: run { running = false }
                                }
                                stateUpdates.onReceiveCatching { result ->
                                    result.getOrNull()?.let { send(it) } ?: run { running = false }
                                }
                            }
                        }
                    }
                    val subscription = events.subscribe { event ->
                        if (event.invalidatesSnapshot()) {
                            revision.incrementAndGet()
                            stateUpdates.trySend(
                                JSONObject()
                                    .put("event", "state.changed")
                                    .put("data", snapshot())
                                    .toString(),
                            )
                        }
                    }
                    try {
                        for (frame in incoming) {
                            if (frame !is Frame.Text) continue
                            val text = frame.readText()
                            if (text.toByteArray().size > MAX_MESSAGE_BYTES) {
                                close(CloseReason(CloseReason.Codes.TOO_BIG, "Message too large"))
                                break
                            }
                            val response = try {
                                handle(JSONObject(text))
                            } catch (_: Exception) {
                                failure(0, "invalid_request", "Request must be valid JSON")
                            }
                            responses.send(response.toString())
                        }
                    } finally {
                        subscription.close()
                        responses.close()
                        stateUpdates.close()
                        sender.join()
                    }
                }
            }
        }
        started.start(wait = false)
        boundPort = runBlocking { started.engine.resolvedConnectors().single().port }
        server = started
    }

    fun stop() {
        scanStopJob?.cancel()
        scanStopJob = null
        server?.stop(100, 500)
        server = null
        boundPort = 0
    }

    private fun handle(request: JSONObject): JSONObject {
        val id = request.opt("id") as? Number
            ?: return failure(0, "invalid_request", "Request id is required")
        val method = request.optString("method")
        val params = request.optJSONObject("params")
            ?: return failure(id.toInt(), "invalid_request", "params is required")
        fun success(result: Any): JSONObject = JSONObject().put("id", id).put("result", result)
        fun requiredString(name: String): String? =
            params.optString(name).takeIf { params.has(name) && it.isNotEmpty() }

        return try {
            when (method) {
                "state.get" -> success(snapshot())
                "bluetooth.requestPermission" ->
                    success(bluetooth(applicationService.requestBluetoothPermission()))
                "bluetooth.scan" -> {
                    val scanId = applicationService.startDiscovery()
                    val timeoutMs = (params.number("timeoutMs") ?: 5_000L).coerceIn(1L, 60_000L)
                    scanStopJob?.cancel()
                    scanStopJob = CoroutineScope(Dispatchers.Default).launch {
                        delay(timeoutMs)
                        runCatching { applicationService.stopDiscovery(scanId) }
                    }
                    success(accepted())
                }
                "device.connect" -> {
                    val deviceId = requiredString("deviceId")
                        ?: return failure(id.toInt(), "invalid_request", "deviceId is required")
                    val active = applicationService.activeConnectionSnapshot()
                    if (
                        active != null &&
                        (active.deviceId.value != deviceId || active.state != ConnectionState.FAILED)
                    ) {
                        return failure(id.toInt(), "invalid_state", "A different board is already active")
                    }
                    applicationService.connect(DeviceId(deviceId))
                    success(accepted())
                }
                "device.selectController" -> {
                    val deviceId = requiredString("deviceId")
                        ?: return failure(id.toInt(), "invalid_request", "deviceId is required")
                    val rawCanId = params.opt("canId") as? Number
                        ?: return failure(id.toInt(), "invalid_request", "canId is required")
                    val numericCanId = rawCanId.toDouble()
                    if (
                        !numericCanId.isFinite() ||
                        numericCanId % 1.0 != 0.0 ||
                        numericCanId.toInt() !in 0..253
                    ) {
                        return failure(id.toInt(), "invalid_request", "CAN ID must be an integer in 0...253")
                    }
                    applicationService.selectController(DeviceId(deviceId), numericCanId.toInt())
                    success(accepted())
                }
                "device.disconnect" -> {
                    applicationService.disconnect()
                    success(accepted())
                }
                "settings.update" -> {
                    val expected = params.opt("expectedRevision") as? Number
                        ?: return failure(id.toInt(), "invalid_request", "expectedRevision is required")
                    val patch = params.optJSONObject("patch")
                        ?: return failure(id.toInt(), "invalid_request", "patch is required")
                    success(settings(applicationService.updateSettings(expected.toInt(), patch.toString())))
                }
                "savedDevice.update" -> {
                    val savedId = requiredString("savedDeviceId")
                        ?: return failure(id.toInt(), "invalid_request", "savedDeviceId is required")
                    val expected = params.opt("expectedRevision") as? Number
                        ?: return failure(id.toInt(), "invalid_request", "expectedRevision is required")
                    if (!params.has("customDisplayName")) {
                        return failure(id.toInt(), "invalid_request", "customDisplayName is required")
                    }
                    val name = if (params.isNull("customDisplayName")) {
                        null
                    } else {
                        params.opt("customDisplayName") as? String
                            ?: return failure(id.toInt(), "invalid_request", "customDisplayName is invalid")
                    }
                    success(savedDevice(applicationService.updateSavedDevice(savedId, expected.toInt(), name)))
                }
                "boardProfile.get" -> {
                    val savedId = requiredString("savedDeviceId")
                        ?: return failure(id.toInt(), "invalid_request", "savedDeviceId is required")
                    success(boardProfile(applicationService.boardProfile(savedId)))
                }
                "boardProfile.update" -> {
                    val savedId = requiredString("savedDeviceId")
                        ?: return failure(id.toInt(), "invalid_request", "savedDeviceId is required")
                    val expected = params.opt("expectedRevision") as? Number
                        ?: return failure(id.toInt(), "invalid_request", "expectedRevision is required")
                    val patch = params.optJSONObject("patch")
                        ?: return failure(id.toInt(), "invalid_request", "patch is required")
                    success(
                        boardProfile(
                            applicationService.updateBoardProfile(
                                savedId,
                                expected.toInt(),
                                patch.toString(),
                            ),
                        ),
                    )
                }
                "session.get" -> {
                    val sessionId = requiredString("sessionId")
                        ?: return failure(id.toInt(), "invalid_request", "sessionId is required")
                    val summary = applicationService.session(sessionId)
                    val maxPoints = (params.optInt("maxPoints", 400)).coerceIn(1, MAX_SERIES_POINTS)
                    success(
                        JSONObject()
                            .put("summary", session(summary))
                            .put(
                                "series",
                                JSONArray(
                                    applicationService.querySessionSeries(
                                        sessionId,
                                        0,
                                        summary.totalElapsedMs,
                                        listOf("packVoltageV", "dutyCycle", "speedKph"),
                                        maxPoints,
                                    ).map(::seriesPoint),
                                ),
                            ),
                    )
                }
                "session.getDiagnostics" -> {
                    val sessionId = requiredString("sessionId")
                        ?: return failure(id.toInt(), "invalid_request", "sessionId is required")
                    val center = params.number("centerElapsedMs")
                        ?: return failure(id.toInt(), "invalid_request", "centerElapsedMs is required")
                    val before = params.number("beforeMs")
                        ?: return failure(id.toInt(), "invalid_request", "beforeMs is required")
                    val after = params.number("afterMs")
                        ?: return failure(id.toInt(), "invalid_request", "afterMs is required")
                    success(
                        JSONArray(
                            applicationService.diagnosticWindow(
                                sessionId,
                                center,
                                before,
                                after,
                                DIAGNOSTIC_FIELDS,
                            ).map(::seriesPoint),
                        ),
                    )
                }
                "session.export" -> {
                    val sessionId = requiredString("sessionId")
                        ?: return failure(id.toInt(), "invalid_request", "sessionId is required")
                    val format = requiredString("format")
                        ?: return failure(id.toInt(), "invalid_request", "format is required")
                    val value = applicationService.exportSession(sessionId, format)
                    success(
                        JSONObject()
                            .put("fileName", value.fileName)
                            .put("delivery", if (exportPresenter?.invoke(value) == true) "presented" else "staged"),
                    )
                }
                "session.delete" -> {
                    val sessionId = requiredString("sessionId")
                        ?: return failure(id.toInt(), "invalid_request", "sessionId is required")
                    applicationService.deleteSession(sessionId)
                    success(accepted())
                }
                "sessionGroup.unlink" -> {
                    val groupId = requiredString("sessionGroupId")
                        ?: return failure(id.toInt(), "invalid_request", "sessionGroupId is required")
                    applicationService.unlinkSessionGroup(groupId)
                    success(accepted())
                }
                else -> failure(id.toInt(), "invalid_request", "Unknown method")
            }
        } catch (_: IllegalArgumentException) {
            failure(id.toInt(), "not_found", "Requested value was not found")
        } catch (_: IllegalStateException) {
            failure(id.toInt(), "invalid_state", "Operation is not valid in the current state")
        } catch (error: ProductStoreException) {
            when (error) {
                is ProductStoreException.ProfileRevisionConflict,
                is ProductStoreException.SavedDeviceRevisionConflict,
                is ProductStoreException.SettingsRevisionConflict,
                -> failure(id.toInt(), "conflict", "Revision conflict")
                ProductStoreException.SavedDeviceNotFound ->
                    failure(id.toInt(), "not_found", "Saved device was not found")
                ProductStoreException.InvalidProfilePatch,
                ProductStoreException.InvalidSavedDevicePatch,
                ProductStoreException.InvalidSettingsPatch,
                -> failure(id.toInt(), "invalid_request", "Patch is invalid")
            }
        } catch (error: SessionStoreException) {
            when (error) {
                SessionStoreException.SessionNotFound ->
                    failure(id.toInt(), "not_found", "Session was not found")
                SessionStoreException.SessionActive ->
                    failure(id.toInt(), "invalid_state", "Session is active")
                else -> failure(id.toInt(), "invalid_request", "Session request is invalid")
            }
        } catch (_: IllegalStateException) {
            failure(id.toInt(), "invalid_state", "Backend state does not permit the operation")
        } catch (_: Exception) {
            failure(id.toInt(), "internal", "Backend operation failed")
        }
    }

    private fun snapshot(): JSONObject {
        val discovery = applicationService.discoverySnapshot()
        val sessions = runCatching {
            applicationService.sessions(SessionListQuery(limit = MAX_RECENT_ITEMS))
        }.getOrNull()
        val active = applicationService.activeConnectionSnapshot()
        val firmware = applicationService.firmwareSnapshot()
        val packageVersion = applicationService.refloatPackageVersionSnapshot()
        val activePayload = active?.let(::connection)
        if (activePayload != null) {
            activePayload.put(
                "firmware",
                if (firmware != null && packageVersion != null) {
                    JSONObject()
                        .put("vescVersion", "${firmware.major}.${firmware.minor.toString().padStart(2, '0')}")
                        .put("packageName", "refloat")
                        .put("packageVersion", packageVersion)
                        .put("hardwareName", firmware.hardwareName ?: JSONObject.NULL)
                        .put(
                            "supported",
                            RefloatCompatibility.entry(packageVersion) != null &&
                                VescCompatibility.supports(firmware),
                        )
                } else {
                    JSONObject.NULL
                },
            )
        }
        return JSONObject()
            .put("revision", revision.get())
            .put("bluetooth", bluetooth(discovery.bluetoothState, discovery.activeScanId != null))
            .put("discoveredDevices", JSONArray(discovery.knownDevices.take(MAX_RECENT_ITEMS).map(::device)))
            .put(
                "savedDevices",
                JSONArray(runCatching { applicationService.savedDevices() }.getOrDefault(emptyList())
                    .take(MAX_RECENT_ITEMS).map(::savedDevice)),
            )
            .put("activeConnection", activePayload ?: JSONObject.NULL)
            .put("recentSessions", JSONArray(sessions?.items?.map(::session) ?: emptyList<JSONObject>()))
            .put(
                "sessionGroups",
                JSONArray(runCatching { applicationService.sessionGroups() }.getOrDefault(emptyList())
                    .take(MAX_RECENT_ITEMS).map(::sessionGroup)),
            )
            .put("settings", runCatching { settings(applicationService.settings()) }.getOrDefault(JSONObject()))
    }

    private fun bluetooth(value: BluetoothState, scanning: Boolean = false) = JSONObject()
        .put("radio", value.radio.name.lowercase())
        .put("scanPermission", value.scanPermission.name.lowercase())
        .put("connectPermission", value.connectPermission.name.lowercase())
        .put("scanning", scanning)

    private fun device(value: DiscoveredDevice) = JSONObject()
        .put("deviceId", value.id.value)
        .put("name", value.advertisedName ?: "")
        .put("state", "discovered")
        .put("lastSeenAt", value.lastSeenAt.toString())

    private fun connection(value: ActiveConnectionSnapshot) = JSONObject()
        .put("deviceId", value.deviceId.value)
        .put("savedDeviceId", value.savedDevice?.savedDeviceId ?: JSONObject.NULL)
        .put("name", value.savedDevice?.displayName ?: value.advertisedName ?: value.deviceId.value)
        .put("displayName", value.savedDevice?.displayName ?: value.advertisedName ?: value.deviceId.value)
        .put(
            "state",
            when (value.state) {
                ConnectionState.DISCONNECTED -> "disconnected"
                ConnectionState.CONNECTING -> "connecting"
                ConnectionState.CONNECTED -> "connected"
                ConnectionState.DISCOVERING_SERVICES,
                ConnectionState.ENABLING_NOTIFICATIONS,
                -> "discovering_services"
                ConnectionState.IDENTIFYING -> "identifying"
                ConnectionState.SELECTING_CONTROLLER -> "selecting_controller"
                ConnectionState.READY -> "ready"
                ConnectionState.RECONNECTING -> "reconnecting"
                ConnectionState.DISCONNECTING -> "disconnecting"
                ConnectionState.FAILED -> "failed"
            },
        )
        .put(
            "controllerConnection",
            when (val route = value.controllerRoute) {
                ControllerRoute.Direct -> JSONObject().put("kind", "direct")
                is ControllerRoute.Can -> JSONObject()
                    .put("kind", "can")
                    .put("canId", route.canId)
                null -> JSONObject.NULL
            },
        )
        .put(
            "controllerCandidates",
            JSONArray(value.controllerCandidates.map { candidate ->
                JSONObject()
                    .put("canId", candidate.canId)
                    .put("hardwareName", candidate.hardwareName ?: JSONObject.NULL)
                    .put("firmwareName", candidate.firmwareName ?: JSONObject.NULL)
                    .put("vescVersion", candidate.vescVersion)
                    .put("vescSupported", candidate.vescSupported)
            }),
        )
        .put(
            "connectionIssue",
            value.connectionIssue?.let { issue ->
                JSONObject()
                    .put("code", issue.code.name.lowercase())
                    .put("message", issue.message)
            } ?: JSONObject.NULL,
        )
        .put("activeSession", value.activeSession?.let(::session) ?: JSONObject.NULL)
        .put("latestSample", value.latestSample?.let(::liveSample) ?: JSONObject.NULL)

    private fun savedDevice(value: fail.acheron.floatview.backend.SavedDeviceRecord) = JSONObject()
        .put("savedDeviceId", value.savedDeviceId)
        .put("deviceHistoryId", value.deviceHistoryId)
        .put("revision", value.revision)
        .put("displayName", value.displayName)
        .put("customDisplayName", value.customDisplayName ?: JSONObject.NULL)
        .put("advertisedName", value.advertisedName ?: JSONObject.NULL)
        .put("bluetoothIdentifier", value.bluetoothIdentifier)
        .put("lastSeenAt", value.lastSeenAt?.toString() ?: JSONObject.NULL)
        .put("lastConnectedAt", value.lastConnectedAt?.toString() ?: JSONObject.NULL)
        .put("profileComplete", value.profileComplete)

    private fun boardProfile(value: BoardProfileRecord) = JSONObject()
        .put("savedDeviceId", value.savedDeviceId)
        .put("revision", value.revision)
        .put("values", JSONObject(value.valuesJson))
        .put("missingFields", JSONArray(value.missingFields))

    private fun session(value: SessionSummaryRecord) = JSONObject()
        .put("sessionId", value.sessionId)
        .put("deviceHistoryId", value.deviceHistoryId)
        .put("displayName", value.displayName)
        .put("startedAt", value.startedAt.toString())
        .put("endedAt", value.endedAt?.toString() ?: JSONObject.NULL)
        .put("state", value.state)
        .put("integrity", value.integrity)
        .put("sampleCount", value.sampleCount)
        .put("eventCount", value.eventCount)
        .put("ridingElapsedMs", value.ridingElapsedMs)
        .put("totalElapsedMs", value.totalElapsedMs)
        .put("distanceM", value.distanceM ?: JSONObject.NULL)
        .put("maxSpeedMps", value.maxSpeedMps ?: JSONObject.NULL)
        .put("maxDutyCycle", value.maxDutyCycle ?: JSONObject.NULL)

    private fun seriesPoint(value: SessionSeriesPoint): JSONObject {
        val fields = JSONObject()
        value.values.forEach { (key, field) -> fields.put(key, field ?: JSONObject.NULL) }
        return JSONObject().put("elapsedMs", value.elapsedMs).put("values", fields)
    }

    private fun sessionGroup(value: SessionGroupRecord) = JSONObject()
        .put("sessionGroupId", value.sessionGroupId)
        .put("deviceHistoryId", value.deviceHistoryId)
        .put("sessionIds", JSONArray(value.sessionIds))
        .put("totalElapsedMs", value.totalElapsedMs)
        .put("distanceM", value.distanceM ?: JSONObject.NULL)

    private fun settings(value: SettingsRecord) = JSONObject()
        .put("revision", value.revision)
        .put("idleProbeIntervalMs", value.idleProbeIntervalMs)
        .put("idlePersistIntervalMs", value.idlePersistIntervalMs)
        .put("ridingPersistIntervalMs", value.ridingPersistIntervalMs)
        .put("postRideTailMs", value.postRideTailMs)
        .put("reconnectDurationMs", value.reconnectDurationMs)
        .put("automaticSessionLinkingEnabled", value.automaticSessionLinkingEnabled)
        .put("sessionLinkWindowMs", value.sessionLinkWindowMs)
        .put("themePreference", value.themePreference.wireValue)
        .put("unitPreference", value.unitPreference.wireValue)

    private fun liveSample(value: TelemetrySample) = JSONObject()
        .put("sequence", value.sequence)
        .put("observedAt", value.observedAt.toString())
        .put("elapsedMs", value.elapsedMs)
        .put("rideState", value.rideState.name.lowercase())
        .put("boardState", value.boardState ?: JSONObject.NULL)
        .put("packVoltageV", value.packVoltageV ?: JSONObject.NULL)
        .put("dutyCycle", value.dutyCycle ?: JSONObject.NULL)
        .put("controllerTempC", value.controllerTempC ?: JSONObject.NULL)
        .put("motorTempC", value.motorTempC ?: JSONObject.NULL)
        .put("footpadState", value.footpadState ?: JSONObject.NULL)
        .put("motorCurrentA", value.extension["motorCurrentA"] ?: JSONObject.NULL)
        .put("batteryCurrentA", value.extension["batteryCurrentA"] ?: JSONObject.NULL)
        .put("erpm", value.extension["erpm"] ?: JSONObject.NULL)
        .put("speedKph", value.extension["speedKph"] ?: JSONObject.NULL)

    private fun failure(id: Int, code: String, message: String) =
        JSONObject().put("id", id).put("error", JSONObject().put("code", code).put("message", message))

    private fun accepted() = JSONObject().put("accepted", true)
    private fun JSONObject.number(name: String): Long? = (opt(name) as? Number)?.toLong()

    private fun Any.invalidatesSnapshot(): Boolean =
        this is BluetoothState ||
            this is DiscoverySnapshot ||
            this is ConnectionState ||
            this is DiscoveredDevice ||
            this is SessionRecord ||
            this is BoardProfileRecord ||
            this is fail.acheron.floatview.backend.SavedDeviceRecord ||
            this is SettingsRecord ||
            this is TelemetrySample ||
            this is VescProtocolMessage.Firmware ||
            this is VescProtocolMessage.RefloatInfo

    private companion object {
        const val MAX_MESSAGE_BYTES = 65_536
        const val MAX_RECENT_ITEMS = 50
        const val MAX_SERIES_POINTS = 2_000
        val DIAGNOSTIC_FIELDS = listOf(
            "packVoltageV", "dutyCycle", "controllerTempC", "motorTempC", "boardState",
            "footpadState", "motorCurrentA", "batteryCurrentA", "erpm", "speedKph",
        )
    }
}
