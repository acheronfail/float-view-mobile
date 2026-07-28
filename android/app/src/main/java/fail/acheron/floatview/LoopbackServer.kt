package fail.acheron.floatview

import android.content.res.AssetManager
import android.util.Base64
import fail.acheron.floatview.core.ApplicationService
import fail.acheron.floatview.core.ActiveConnectionSnapshot
import fail.acheron.floatview.core.BluetoothState
import fail.acheron.floatview.core.BoardProfileRecord
import fail.acheron.floatview.core.ConnectionState
import fail.acheron.floatview.core.DiscoveredDevice
import fail.acheron.floatview.core.EventHub
import fail.acheron.floatview.core.ProductStoreException
import fail.acheron.floatview.core.SavedDeviceRecord
import fail.acheron.floatview.core.SessionEventRecord
import fail.acheron.floatview.core.SessionExportRecord
import fail.acheron.floatview.core.SessionGroupRecord
import fail.acheron.floatview.core.SessionListQuery
import fail.acheron.floatview.core.SessionSeriesPoint
import fail.acheron.floatview.core.SessionStoreException
import fail.acheron.floatview.core.SessionSummaryRecord
import fail.acheron.floatview.core.SettingsRecord
import fail.acheron.floatview.core.SessionRecord
import java.security.SecureRandom
import java.time.Instant
import io.ktor.http.ContentType
import io.ktor.http.HttpStatusCode
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
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.launch
import org.json.JSONArray
import org.json.JSONObject

class LoopbackServer(
    private val assets: AssetManager,
    private val applicationService: ApplicationService? = null,
    private val exportPresenter: ((SessionExportRecord) -> Boolean)? = null,
    private val events: EventHub? = null,
) {
    private val token = ByteArray(32).also(SecureRandom()::nextBytes)
        .let { Base64.encodeToString(it, Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING) }
    private var server: EmbeddedServer<*, *>? = null
    private var boundPort = 0
    val port: Int get() = boundPort
    val origin: String get() = "http://127.0.0.1:$port"

    fun start() {
        if (server != null) return
        val started = embeddedServer(CIO, host = "127.0.0.1", port = 0) {
            install(WebSockets) { maxFrameSize = 65_536 }
            routing {
                get("/api/bootstrap") {
                    call.respondText(
                        JSONObject()
                            .put("apiVersion", JSONObject().put("major", 1).put("minor", 0))
                            .put("webSocketUrl", "ws://127.0.0.1:$port/api/v1/ws")
                            .put("sessionToken", token)
                            .toString(),
                        ContentType.Application.Json,
                    )
                }
                webSocket("/api/v1/ws") {
                    if (
                        call.request.queryParameters["token"] != token ||
                        call.request.header("Origin") != origin
                    ) {
                        close(CloseReason(CloseReason.Codes.VIOLATED_POLICY, "Forbidden"))
                        return@webSocket
                    }
                    // Bound memory without dropping request responses. State
                    // notifications are best-effort and naturally coalesce
                    // while one message is queued.
                    val messages = Channel<String>(capacity = 1)
                    val forwarding = launch {
                        for (message in messages) send(message)
                    }
                    val subscription = events?.subscribe { event ->
                        if (event.invalidatesSnapshot()) {
                            messages.trySend(SNAPSHOT_CHANGED_NOTIFICATION)
                        }
                    }
                    var initialized = false
                    try {
                        for (frame in incoming) {
                            if (frame !is Frame.Text) continue
                            val request = try {
                                JSONObject(frame.readText())
                            } catch (_: Exception) {
                                close(CloseReason(CloseReason.Codes.CANNOT_ACCEPT, "Invalid JSON"))
                                return@webSocket
                            }
                            val response = handleRequest(request, initialized)
                            if (
                                request.optString("method") == "system.hello" &&
                                !response.has("error")
                            ) {
                                initialized = true
                            }
                            messages.send(response.toString())
                        }
                    } finally {
                        subscription?.close()
                        messages.close()
                        forwarding.join()
                    }
                }
                get("/{path...}") {
                    val relative = call.parameters.getAll("path")
                        ?.joinToString("/")
                        ?.ifEmpty { "index.html" }
                        ?: "index.html"
                    if (relative.split('/').any { it == ".." }) {
                        call.respondBytes(ByteArray(0), status = HttpStatusCode.Forbidden)
                        return@get
                    }
                    val body = try {
                        assets.open("web/$relative").use { it.readBytes() }
                    } catch (_: Exception) {
                        call.respondBytes(ByteArray(0), status = HttpStatusCode.NotFound)
                        return@get
                    }
                    val contentType = when (relative.substringAfterLast('.', "")) {
                        "html" -> ContentType.Text.Html
                        "js" -> ContentType.Application.JavaScript
                        "css" -> ContentType.Text.CSS
                        "svg" -> ContentType.Image.SVG
                        else -> ContentType.Application.OctetStream
                    }
                    call.respondBytes(body, contentType)
                }
            }
        }
        started.start(wait = false)
        boundPort = runBlocking { started.engine.resolvedConnectors().single().port }
        server = started
    }

    fun stop() {
        server?.stop(100, 500)
        server = null
        boundPort = 0
    }

    private fun handleRequest(request: JSONObject, initialized: Boolean): JSONObject {
        val id = request.opt("id") ?: JSONObject.NULL
        val method = request.optString("method")
        fun success(result: Any) = JSONObject().put("jsonrpc", "2.0").put("id", id).put("result", result)
        fun failure(code: Int, kind: String, message: String) = JSONObject()
            .put("jsonrpc", "2.0").put("id", id)
            .put(
                "error",
                JSONObject().put("code", code).put("message", message)
                    .put("data", JSONObject().put("kind", kind).put("recoverable", false)),
            )
        if (method == "system.hello") {
            val api = request.optJSONObject("params")?.optJSONObject("api")
            if (api?.optInt("major") != 1 || api.optInt("minMinor") > 0) {
                return failure(-32001, "unsupported", "Unsupported API version")
            }
            return success(
                JSONObject()
                    .put("api", JSONObject().put("major", 1).put("minor", 0))
                    .put(
                        "backend",
                        JSONObject().put("name", "float-view-android").put("version", "0.1.0")
                            .put("platform", "android"),
                    )
                    .put(
                        "capabilities",
                        JSONArray()
                            .put(JSONObject().put("name", "system.snapshot").put("available", true))
                            .put(
                                JSONObject().put("name", "bluetooth.discovery")
                                    .put("available", applicationService != null),
                            )
                            .put(
                                JSONObject().put("name", "device.connection")
                                    .put("available", applicationService != null),
                            )
                            .put(
                                JSONObject().put("name", "boardProfiles.local")
                                    .put("available", applicationService != null),
                            )
                            .put(
                                JSONObject().put("name", "sessions.review")
                                    .put("available", applicationService != null),
                            )
                            .put(
                                JSONObject().put("name", "sessions.manage")
                                    .put("available", applicationService != null),
                            )
                            .put(
                                JSONObject().put("name", "sessions.export")
                                    .put("available", applicationService != null),
                            )
                            .put(
                                JSONObject().put("name", "settings.local")
                                    .put("available", applicationService != null),
                            ),
                    )
                    .put(
                        "limits",
                        JSONObject().put("minTelemetryIntervalMs", 50).put("maxMessageBytes", 65_536)
                            .put("maxReconnectDurationMs", 3_600_000),
                    )
                    .put("snapshot", snapshot()),
            )
        }
        if (!initialized) return failure(-32000, "invalid_state", "system.hello required")
        if (method == "system.getSnapshot") return success(snapshot())
        if (method == "bluetooth.getState") return success(bluetooth())
        if (method == "bluetooth.requestPermission") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Bluetooth is unavailable")
            return success(bluetooth(service.requestBluetoothPermission()))
        }
        if (method == "discovery.start") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Discovery is unavailable")
            return try {
                success(JSONObject().put("scanId", service.startDiscovery()))
            } catch (error: IllegalStateException) {
                val state = service.discoverySnapshot().bluetoothState
                if (
                    state.scanPermission.name != "GRANTED" &&
                    state.scanPermission.name != "NOT_REQUIRED"
                ) {
                    failure(-32003, "permission_required", "Bluetooth scan permission is required")
                } else {
                    failure(-32000, "invalid_state", "Bluetooth is not powered on")
                }
            }
        }
        if (method == "discovery.stop") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Discovery is unavailable")
            val scanId = request.optJSONObject("params")?.optString("scanId").orEmpty()
            if (scanId.isEmpty()) return failure(-32602, "protocol_error", "scanId is required")
            return try {
                service.stopDiscovery(scanId)
                success(JSONObject().put("accepted", true))
            } catch (_: IllegalArgumentException) {
                failure(-32000, "invalid_state", "Unknown active scan")
            }
        }
        if (method == "device.connect") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Connections are unavailable")
            val deviceId = request.optJSONObject("params")?.optString("deviceId").orEmpty()
            if (deviceId.isEmpty()) {
                return failure(-32602, "protocol_error", "deviceId is required")
            }
            return try {
                service.connect(fail.acheron.floatview.core.DeviceId(deviceId))
                val active = service.activeConnectionSnapshot()
                    ?: return failure(-32004, "connection_failed", "Connection did not start")
                success(connection(active))
            } catch (error: IllegalArgumentException) {
                if (error.message == "Unknown device") {
                    failure(-32002, "not_found", "The discovered board is no longer available")
                } else {
                    failure(-32000, "invalid_state", "Another board is active")
                }
            } catch (_: Exception) {
                failure(-32004, "connection_failed", "Unable to connect")
            }
        }
        if (method == "device.disconnect") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Connections are unavailable")
            return try {
                service.disconnect()
                success(JSONObject().put("accepted", true))
            } catch (_: Exception) {
                failure(-32004, "connection_failed", "Unable to disconnect cleanly")
            }
        }
        if (method == "device.getActive") {
            val active = applicationService?.activeConnectionSnapshot()
                ?: return success(JSONObject.NULL)
            return success(connection(active))
        }
        if (method == "vesc.getFirmwareInfo") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "VESC protocol is unavailable")
            return try {
                val firmware = service.requestFirmwareInfo()
                    ?: return failure(
                        -32007,
                        "device_not_ready",
                        "Waiting for the VESC firmware response",
                    )
                val packageVersion = service.refloatPackageVersionSnapshot()
                    ?: return failure(
                        -32007,
                        "device_not_ready",
                        "Waiting for the Refloat package response",
                    )
                success(
                    JSONObject()
                        .put("vescVersion", "${firmware.major}.${firmware.minor.toString().padStart(2, '0')}")
                        .put("packageName", "refloat")
                        .put("packageVersion", packageVersion)
                        .put("hardwareName", firmware.hardwareName ?: JSONObject.NULL)
                        .put(
                            "supported",
                            (packageVersion == "1.2" || packageVersion.startsWith("1.2.")) &&
                                (firmware.major > 6 || (firmware.major == 6 && firmware.minor >= 2)),
                        ),
                )
            } catch (_: IllegalStateException) {
                failure(-32007, "device_not_ready", "The VESC connection is not ready")
            }
        }
        if (method == "savedDevices.list") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Saved devices are unavailable")
            return try {
                success(JSONArray(service.savedDevices().map(::savedDevice)))
            } catch (_: Exception) {
                failure(-32005, "storage_unavailable", "Unable to read saved devices")
            }
        }
        if (method == "boardProfiles.get") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Board Profiles are unavailable")
            val savedDeviceId = request.optJSONObject("params")
                ?.optString("savedDeviceId")
                .orEmpty()
            if (savedDeviceId.isEmpty()) {
                return failure(-32602, "protocol_error", "savedDeviceId is required")
            }
            return try {
                success(boardProfile(service.boardProfile(savedDeviceId)))
            } catch (error: ProductStoreException) {
                productStoreFailure(error, ::failure)
            } catch (_: Exception) {
                failure(-32005, "storage_unavailable", "Unable to read Board Profile")
            }
        }
        if (method == "boardProfiles.update") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Board Profiles are unavailable")
            val params = request.optJSONObject("params")
                ?: return failure(-32602, "protocol_error", "Board Profile parameters are required")
            val savedDeviceId = params.optString("savedDeviceId")
            val expectedRevision = params.opt("expectedRevision")
            val patch = params.optJSONObject("patch")
            if (
                savedDeviceId.isEmpty() ||
                expectedRevision !is Number ||
                expectedRevision.toInt() < 0 ||
                patch == null
            ) {
                return failure(
                    -32602,
                    "protocol_error",
                    "savedDeviceId, expectedRevision, and an object patch are required",
                )
            }
            return try {
                success(
                    boardProfile(
                        service.updateBoardProfile(
                            savedDeviceId,
                            expectedRevision.toInt(),
                            patch.toString(),
                        ),
                    ),
                )
            } catch (error: ProductStoreException) {
                productStoreFailure(error, ::failure)
            } catch (_: Exception) {
                failure(-32005, "storage_unavailable", "Unable to update Board Profile")
            }
        }
        if (method == "settings.get") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Settings are unavailable")
            return try {
                success(settings(service.settings()))
            } catch (error: ProductStoreException) {
                productStoreFailure(error, ::failure)
            } catch (_: Exception) {
                failure(-32005, "storage_unavailable", "Unable to read settings")
            }
        }
        if (method == "settings.update") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Settings are unavailable")
            val params = request.optJSONObject("params")
                ?: return failure(-32602, "protocol_error", "Settings parameters are required")
            val expectedRevision = params.opt("expectedRevision")
            val patch = params.optJSONObject("patch")
            if (expectedRevision !is Number || expectedRevision.toInt() < 0 || patch == null) {
                return failure(
                    -32602,
                    "protocol_error",
                    "expectedRevision and an object patch are required",
                )
            }
            return try {
                success(settings(service.updateSettings(expectedRevision.toInt(), patch.toString())))
            } catch (error: ProductStoreException) {
                productStoreFailure(error, ::failure)
            } catch (_: Exception) {
                failure(-32005, "storage_unavailable", "Unable to update settings")
            }
        }
        if (method == "sessions.list") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Sessions are unavailable")
            val params = request.optJSONObject("params") ?: JSONObject()
            val filters = params.optJSONObject("filters") ?: JSONObject()
            val sort = params.optJSONObject("sort") ?: JSONObject()
            val limit = params.optInt("limit", 20)
            if (limit !in 1..100) {
                return failure(-32602, "protocol_error", "limit must be between 1 and 100")
            }
            return try {
                val page = service.sessions(
                    SessionListQuery(
                        cursor = params.optionalString("cursor"),
                        limit = limit,
                        deviceHistoryId = filters.optionalString("deviceHistoryId"),
                        startedAfter = filters.optionalInstant("startedAfter"),
                        startedBefore = filters.optionalInstant("startedBefore"),
                        minimumTotalElapsedMs = filters.optionalLong("minimumTotalElapsedMs"),
                        maximumTotalElapsedMs = filters.optionalLong("maximumTotalElapsedMs"),
                        minimumRidingElapsedMs = filters.optionalLong("minimumRidingElapsedMs"),
                        maximumRidingElapsedMs = filters.optionalLong("maximumRidingElapsedMs"),
                        minimumDistanceM = filters.optionalDouble("minimumDistanceM"),
                        maximumDistanceM = filters.optionalDouble("maximumDistanceM"),
                        state = filters.optionalString("state"),
                        integrity = filters.optionalString("integrity"),
                        sortField = sort.optString("field", "startedAt"),
                        sortDescending = sort.optString("direction", "desc") == "desc",
                    ),
                )
                success(
                    JSONObject()
                        .put("items", JSONArray(page.items.map(::session)))
                        .put("nextCursor", page.nextCursor ?: JSONObject.NULL),
                )
            } catch (error: SessionStoreException) {
                sessionStoreFailure(error, ::failure)
            } catch (_: Exception) {
                failure(-32005, "storage_unavailable", "Unable to list sessions")
            }
        }
        if (method == "sessions.get" || method == "sessions.getSummary") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Sessions are unavailable")
            val sessionId = request.optJSONObject("params")?.optString("sessionId").orEmpty()
            if (sessionId.isEmpty()) {
                return failure(-32602, "protocol_error", "sessionId is required")
            }
            return try {
                success(session(service.session(sessionId)))
            } catch (error: SessionStoreException) {
                sessionStoreFailure(error, ::failure)
            } catch (_: Exception) {
                failure(-32005, "storage_unavailable", "Unable to read session")
            }
        }
        if (method == "sessions.querySeries") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Session queries are unavailable")
            val params = request.optJSONObject("params")
                ?: return failure(-32602, "protocol_error", "Invalid series query")
            val fields = params.optJSONArray("fields")?.strings()
                ?: return failure(-32602, "protocol_error", "fields are required")
            return try {
                success(
                    JSONArray(
                        service.querySessionSeries(
                            params.optString("sessionId"),
                            params.getLong("startElapsedMs"),
                            params.getLong("endElapsedMs"),
                            fields,
                            params.getInt("maxPoints"),
                        ).map(::seriesPoint),
                    ),
                )
            } catch (error: SessionStoreException) {
                sessionStoreFailure(error, ::failure)
            } catch (_: Exception) {
                failure(-32602, "protocol_error", "Invalid series query")
            }
        }
        if (method == "sessions.listEvents") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Session queries are unavailable")
            val params = request.optJSONObject("params")
                ?: return failure(-32602, "protocol_error", "Invalid event query")
            return try {
                success(
                    JSONArray(
                        service.sessionEvents(
                            params.optString("sessionId"),
                            params.getLong("startElapsedMs"),
                            params.getLong("endElapsedMs"),
                        ).map(::sessionEvent),
                    ),
                )
            } catch (error: SessionStoreException) {
                sessionStoreFailure(error, ::failure)
            } catch (_: Exception) {
                failure(-32602, "protocol_error", "Invalid event query")
            }
        }
        if (method == "sessions.getDiagnosticWindow") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Session queries are unavailable")
            val params = request.optJSONObject("params")
                ?: return failure(-32602, "protocol_error", "Invalid diagnostic query")
            val fields = params.optJSONArray("fields")?.strings()
                ?: return failure(-32602, "protocol_error", "fields are required")
            return try {
                success(
                    JSONArray(
                        service.diagnosticWindow(
                            params.optString("sessionId"),
                            params.getLong("centerElapsedMs"),
                            params.getLong("beforeMs"),
                            params.getLong("afterMs"),
                            fields,
                        ).map(::seriesPoint),
                    ),
                )
            } catch (error: SessionStoreException) {
                sessionStoreFailure(error, ::failure)
            } catch (_: Exception) {
                failure(-32602, "protocol_error", "Invalid diagnostic query")
            }
        }
        if (method == "sessions.export") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Session export is unavailable")
            val params = request.optJSONObject("params")
                ?: return failure(-32602, "protocol_error", "sessionId and format are required")
            val sessionId = params.optString("sessionId")
            val format = params.optString("format")
            if (sessionId.isEmpty() || format.isEmpty()) {
                return failure(-32602, "protocol_error", "sessionId and format are required")
            }
            return try {
                val value = service.exportSession(sessionId, format)
                val delivery = if (exportPresenter?.invoke(value) == true) "presented" else "staged"
                success(
                    JSONObject()
                        .put("exportId", value.exportId)
                        .put("fileName", value.fileName)
                        .put("delivery", delivery),
                )
            } catch (error: SessionStoreException) {
                sessionStoreFailure(error, ::failure)
            } catch (_: Exception) {
                failure(-32005, "storage_unavailable", "Unable to export session")
            }
        }
        if (method == "sessions.delete") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Session deletion is unavailable")
            val sessionId = request.optJSONObject("params")?.optString("sessionId").orEmpty()
            if (sessionId.isEmpty()) {
                return failure(-32602, "protocol_error", "sessionId is required")
            }
            return try {
                service.deleteSession(sessionId)
                success(JSONObject().put("accepted", true))
            } catch (error: SessionStoreException) {
                sessionStoreFailure(error, ::failure)
            } catch (_: Exception) {
                failure(-32005, "storage_unavailable", "Unable to delete session")
            }
        }
        if (method == "sessionGroups.list") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Session groups are unavailable")
            return try {
                success(JSONArray(service.sessionGroups().map(::sessionGroup)))
            } catch (_: Exception) {
                failure(-32005, "storage_unavailable", "Unable to list session groups")
            }
        }
        if (method == "sessionGroups.get") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Session groups are unavailable")
            val groupId = request.optJSONObject("params")?.optString("sessionGroupId").orEmpty()
            if (groupId.isEmpty()) {
                return failure(-32602, "protocol_error", "sessionGroupId is required")
            }
            return try {
                success(sessionGroup(service.sessionGroup(groupId)))
            } catch (error: SessionStoreException) {
                sessionStoreFailure(error, ::failure)
            } catch (_: Exception) {
                failure(-32005, "storage_unavailable", "Unable to read session group")
            }
        }
        if (method == "sessionGroups.unlink") {
            val service = applicationService
                ?: return failure(-32601, "unsupported", "Session groups are unavailable")
            val groupId = request.optJSONObject("params")?.optString("sessionGroupId").orEmpty()
            if (groupId.isEmpty()) {
                return failure(-32602, "protocol_error", "sessionGroupId is required")
            }
            return try {
                service.unlinkSessionGroup(groupId)
                success(JSONObject().put("accepted", true))
            } catch (_: Exception) {
                failure(-32005, "storage_unavailable", "Unable to unlink session group")
            }
        }
        return failure(-32601, "unsupported", "Unknown method: $method")
    }

    private fun snapshot(): JSONObject {
        val discovery = applicationService?.discoverySnapshot()
        return JSONObject()
        .put("revision", 0)
        .put("bluetooth", discovery?.let { bluetooth(it.bluetoothState) } ?: bluetooth())
        .put(
            "knownDevices",
            JSONArray(discovery?.knownDevices?.map(::device) ?: emptyList<JSONObject>()),
        )
        .put(
            "activeConnection",
            applicationService?.activeConnectionSnapshot()?.let(::connection) ?: JSONObject.NULL,
        )
        .put("telemetrySubscriptions", JSONArray())
        .put("loggingJobs", JSONArray())
    }

    private fun bluetooth(
        state: BluetoothState? = applicationService?.discoverySnapshot()?.bluetoothState,
    ) = JSONObject()
        .put("radio", state?.radio?.name?.lowercase() ?: "unknown")
        .put("scanPermission", state?.scanPermission?.name?.lowercase() ?: "unknown")
        .put("connectPermission", state?.connectPermission?.name?.lowercase() ?: "unknown")

    private fun device(value: DiscoveredDevice) = JSONObject()
        .put("deviceId", value.id.value)
        .put("name", value.advertisedName ?: "")
        .put("state", "discovered")
        .put("lastSeenAt", value.lastSeenAt.toString())

    private fun connection(value: ActiveConnectionSnapshot) = JSONObject()
        .put("deviceId", value.deviceId.value)
        .put("name", value.advertisedName ?: "")
        .put(
            "state",
            when (value.state) {
                ConnectionState.DISCONNECTED -> "disconnected"
                ConnectionState.CONNECTING -> "connecting"
                ConnectionState.CONNECTED -> "connected"
                ConnectionState.DISCOVERING_SERVICES,
                ConnectionState.ENABLING_NOTIFICATIONS,
                -> "discovering_services"
                ConnectionState.READY -> "ready"
                ConnectionState.RECONNECTING -> "reconnecting"
                ConnectionState.DISCONNECTING -> "disconnecting"
                ConnectionState.FAILED -> "failed"
            },
        )

    private fun savedDevice(value: SavedDeviceRecord) = JSONObject()
        .put("savedDeviceId", value.savedDeviceId)
        .put("deviceHistoryId", value.deviceHistoryId)
        .put("displayName", value.displayName)
        .put("advertisedName", value.advertisedName ?: JSONObject.NULL)
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

    private fun sessionEvent(value: SessionEventRecord) = JSONObject()
        .put("eventId", value.eventId)
        .put("elapsedMs", value.elapsedMs)
        .put("kind", value.kind)
        .put("details", JSONObject(value.detailsJson))

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
        .put("rawPacketCaptureEnabled", value.rawPacketCaptureEnabled)

    private fun sessionStoreFailure(
        error: SessionStoreException,
        failure: (Int, String, String) -> JSONObject,
    ): JSONObject = when (error) {
        SessionStoreException.SessionNotFound ->
            failure(-32004, "device_not_found", "Session not found")
        SessionStoreException.SessionActive ->
            failure(-32000, "invalid_state", "Active sessions cannot be exported or deleted")
        SessionStoreException.InvalidCursor,
        SessionStoreException.InvalidRange,
        -> failure(-32602, "protocol_error", "Invalid session query")
        SessionStoreException.InvalidExportFormat ->
            failure(-32602, "protocol_error", "Invalid session export format")
        is SessionStoreException.UnsupportedField ->
            failure(-32602, "protocol_error", "Unsupported session field: ${error.field}")
    }

    private fun JSONObject.optionalString(key: String): String? =
        if (!has(key) || isNull(key)) null else optString(key).takeIf(String::isNotEmpty)

    private fun JSONObject.optionalInstant(key: String): Instant? =
        optionalString(key)?.let {
            try {
                Instant.parse(it)
            } catch (_: Exception) {
                null
            }
        }

    private fun JSONObject.optionalLong(key: String): Long? =
        if (!has(key) || isNull(key)) null else opt(key).let { it as? Number }?.toLong()

    private fun JSONObject.optionalDouble(key: String): Double? =
        if (!has(key) || isNull(key)) null else opt(key).let { it as? Number }?.toDouble()

    private fun JSONArray.strings(): List<String> = buildList {
        for (index in 0 until length()) add(getString(index))
    }

    private fun productStoreFailure(
        error: ProductStoreException,
        failure: (Int, String, String) -> JSONObject,
    ): JSONObject = when (error) {
        ProductStoreException.SavedDeviceNotFound ->
            failure(-32004, "device_not_found", "Saved device not found")
        is ProductStoreException.ProfileRevisionConflict ->
            failure(-32000, "invalid_state", "Profile revision conflict")
        ProductStoreException.InvalidProfilePatch ->
            failure(-32602, "protocol_error", "Invalid Board Profile patch")
        is ProductStoreException.SettingsRevisionConflict ->
            failure(-32000, "invalid_state", "Settings revision conflict")
        ProductStoreException.InvalidSettingsPatch ->
            failure(-32602, "protocol_error", "Invalid Settings patch")
    }

    private fun Any.invalidatesSnapshot(): Boolean =
        this is BluetoothState ||
            this is ConnectionState ||
            this is DiscoveredDevice ||
            this is SessionRecord ||
            this is BoardProfileRecord ||
            this is SettingsRecord

    private companion object {
        val SNAPSHOT_CHANGED_NOTIFICATION = JSONObject()
            .put("jsonrpc", "2.0")
            .put("method", "system.snapshotChanged")
            .put("params", JSONObject())
            .toString()
    }

}
