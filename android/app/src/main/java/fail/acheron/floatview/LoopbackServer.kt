package fail.acheron.floatview

import android.content.res.AssetManager
import android.util.Base64
import fail.acheron.floatview.core.ApplicationService
import fail.acheron.floatview.core.ActiveConnectionSnapshot
import fail.acheron.floatview.core.BluetoothState
import fail.acheron.floatview.core.BoardProfileRecord
import fail.acheron.floatview.core.ConnectionState
import fail.acheron.floatview.core.DiscoveredDevice
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
import java.io.BufferedInputStream
import java.io.BufferedOutputStream
import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket
import java.nio.ByteBuffer
import java.security.MessageDigest
import java.security.SecureRandom
import java.time.Instant
import java.util.concurrent.Executors
import org.json.JSONArray
import org.json.JSONObject

class LoopbackServer(
    private val assets: AssetManager,
    private val applicationService: ApplicationService? = null,
    private val exportPresenter: ((SessionExportRecord) -> Boolean)? = null,
) {
    private val executor = Executors.newCachedThreadPool()
    private val token = ByteArray(32).also(SecureRandom()::nextBytes)
        .let { Base64.encodeToString(it, Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING) }
    private var server: ServerSocket? = null
    val port: Int get() = requireNotNull(server).localPort
    val origin: String get() = "http://127.0.0.1:$port"

    fun start() {
        if (server != null) return
        val socket = ServerSocket(0, 16, InetAddress.getByName("127.0.0.1"))
        server = socket
        executor.execute {
            while (!socket.isClosed) {
                try {
                    val client = socket.accept()
                    executor.execute { handle(client) }
                } catch (_: Exception) {
                    if (!socket.isClosed) continue
                }
            }
        }
    }

    fun stop() {
        server?.close()
        server = null
        executor.shutdownNow()
    }

    private fun handle(socket: Socket) = socket.use {
        val input = BufferedInputStream(it.getInputStream())
        val output = BufferedOutputStream(it.getOutputStream())
        val headerBytes = ArrayList<Byte>()
        var tail = ""
        while (!tail.endsWith("\r\n\r\n") && headerBytes.size <= 65_536) {
            val value = input.read()
            if (value < 0) return
            headerBytes += value.toByte()
            tail = (tail + value.toChar()).takeLast(4)
        }
        val lines = headerBytes.toByteArray().decodeToString().split("\r\n")
        val target = lines.firstOrNull()?.split(' ')?.getOrNull(1) ?: return
        val headers = lines.drop(1).mapNotNull { line ->
            val index = line.indexOf(':')
            if (index < 0) null else line.substring(0, index).lowercase() to
                line.substring(index + 1).trim()
        }.toMap()

        if (target.startsWith("/api/v1/ws")) {
            val queryToken = target.substringAfter("token=", "").substringBefore('&')
            if (
                queryToken != token ||
                headers["origin"] != origin ||
                headers["upgrade"]?.lowercase() != "websocket"
            ) {
                http(output, "403 Forbidden", "text/plain", ByteArray(0))
                return
            }
            val key = headers["sec-websocket-key"] ?: return
            val accept = Base64.encodeToString(
                MessageDigest.getInstance("SHA-1")
                    .digest((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").toByteArray()),
                Base64.NO_WRAP,
            )
            output.write(
                (
                    "HTTP/1.1 101 Switching Protocols\r\n" +
                        "Upgrade: websocket\r\nConnection: Upgrade\r\n" +
                        "Sec-WebSocket-Accept: $accept\r\n\r\n"
                    ).toByteArray(),
            )
            output.flush()
            websocket(input, output)
            return
        }
        if (target == "/api/bootstrap") {
            val body = JSONObject()
                .put("apiVersion", JSONObject().put("major", 1).put("minor", 0))
                .put("webSocketUrl", "ws://127.0.0.1:$port/api/v1/ws")
                .put("sessionToken", token)
                .toString().toByteArray()
            http(output, "200 OK", "application/json", body)
            return
        }
        val relative = if (target == "/") "index.html" else target.substringBefore('?').drop(1)
        if (relative.split('/').any { segment -> segment == ".." }) {
            http(output, "403 Forbidden", "text/plain", ByteArray(0))
            return
        }
        val body = try {
            assets.open("web/$relative").use { stream -> stream.readBytes() }
        } catch (_: Exception) {
            http(output, "404 Not Found", "text/plain", ByteArray(0))
            return
        }
        val type = when (relative.substringAfterLast('.', "")) {
            "html" -> "text/html"
            "js" -> "text/javascript"
            "css" -> "text/css"
            "svg" -> "image/svg+xml"
            else -> "application/octet-stream"
        }
        http(output, "200 OK", type, body)
    }

    private fun websocket(input: BufferedInputStream, output: BufferedOutputStream) {
        var initialized = false
        while (true) {
            val first = input.read()
            val second = input.read()
            if (first < 0 || second < 0) return
            val opcode = first and 0x0f
            var length = second and 0x7f
            if (length == 126) {
                val extendedLength = readExactly(input, 2) ?: return
                length = ((extendedLength[0].toInt() and 0xff) shl 8) or
                    (extendedLength[1].toInt() and 0xff)
            }
            if (length == 127 || length > 65_536) return
            val mask = if ((second and 0x80) != 0) readExactly(input, 4) ?: return else ByteArray(0)
            val payload = readExactly(input, length) ?: return
            if (mask.isNotEmpty()) payload.indices.forEach { payload[it] =
                (payload[it].toInt() xor mask[it % 4].toInt()).toByte()
            }
            when (opcode) {
                1 -> {
                    val request = try { JSONObject(payload.decodeToString()) } catch (_: Exception) { return }
                    val response = handleRequest(request, initialized)
                    if (request.optString("method") == "system.hello" && !response.has("error")) {
                        initialized = true
                    }
                    frame(output, 1, response.toString().toByteArray())
                }
                8 -> return
                9 -> frame(output, 10, payload)
            }
        }
    }

    private fun readExactly(input: BufferedInputStream, length: Int): ByteArray? {
        val bytes = ByteArray(length)
        var offset = 0
        while (offset < length) {
            val count = input.read(bytes, offset, length - offset)
            if (count < 0) return null
            offset += count
        }
        return bytes
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

    private fun frame(output: BufferedOutputStream, opcode: Int, payload: ByteArray) {
        output.write(0x80 or opcode)
        when {
            payload.size < 126 -> output.write(payload.size)
            payload.size <= 65_535 -> {
                output.write(126)
                output.write(payload.size shr 8)
                output.write(payload.size)
            }
            else -> return
        }
        output.write(payload)
        output.flush()
    }

    private fun http(output: BufferedOutputStream, status: String, type: String, body: ByteArray) {
        output.write(
            (
                "HTTP/1.1 $status\r\nContent-Type: $type\r\nContent-Length: ${body.size}\r\n" +
                    "Cache-Control: no-store\r\nConnection: close\r\n\r\n"
                ).toByteArray(),
        )
        output.write(body)
        output.flush()
    }
}
