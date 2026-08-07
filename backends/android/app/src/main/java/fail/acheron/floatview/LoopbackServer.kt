package fail.acheron.floatview

import android.content.res.AssetManager
import android.util.Base64
import android.util.Log
import fail.acheron.floatview.backend.ActiveConnectionSnapshot
import fail.acheron.floatview.backend.ApplicationService
import fail.acheron.floatview.backend.BluetoothState
import fail.acheron.floatview.backend.BoardProfileRecord
import fail.acheron.floatview.backend.ConnectionState
import fail.acheron.floatview.backend.ControllerActions
import fail.acheron.floatview.backend.DeviceId
import fail.acheron.floatview.backend.DiscoveredDevice
import fail.acheron.floatview.backend.DiscoverySnapshot
import fail.acheron.floatview.backend.EventHub
import fail.acheron.floatview.backend.LocationProviderAuthorization
import fail.acheron.floatview.backend.LocationProviderState
import fail.acheron.floatview.backend.LocationUpdated
import fail.acheron.floatview.backend.LiveLocation
import fail.acheron.floatview.backend.ProductStoreException
import fail.acheron.floatview.backend.RefloatControls
import fail.acheron.floatview.backend.RefloatConfigurationSummary
import fail.acheron.floatview.backend.RefloatConfigurationIdentityRecord
import fail.acheron.floatview.backend.RefloatConfigurationRecord
import fail.acheron.floatview.backend.RefloatMutationException
import fail.acheron.floatview.protocol.refloat.RefloatBooleanControl
import fail.acheron.floatview.backend.SessionExportRecord
import fail.acheron.floatview.backend.SessionEventRecord
import fail.acheron.floatview.backend.SessionElapsedRange
import fail.acheron.floatview.backend.SessionGapPolicyRecord
import fail.acheron.floatview.backend.SessionHistoryCursor
import fail.acheron.floatview.backend.SessionHistoryReducer
import fail.acheron.floatview.backend.SessionPruneMetadataRecord
import fail.acheron.floatview.backend.SessionReadCancellation
import fail.acheron.floatview.backend.SessionMaintenanceJobRecord
import fail.acheron.floatview.backend.SessionListQuery
import fail.acheron.floatview.backend.SessionRecord
import fail.acheron.floatview.backend.SessionSeriesPoint
import fail.acheron.floatview.backend.SessionPublishedSnapshotRecord
import fail.acheron.floatview.backend.SessionStoreException
import fail.acheron.floatview.backend.JobId
import fail.acheron.floatview.backend.SessionSummaryRecord
import fail.acheron.floatview.backend.SettingsRecord
import fail.acheron.floatview.backend.TelemetrySample
import fail.acheron.floatview.backend.TrackingMode
import fail.acheron.floatview.backend.VescProtocolMessage
import fail.acheron.floatview.protocol.refloat.RefloatCompatibility
import fail.acheron.floatview.protocol.refloat.RefloatConfigurationField
import fail.acheron.floatview.protocol.refloat.RefloatConfigurationFieldType
import fail.acheron.floatview.protocol.refloat.RefloatConfigurationLimits
import fail.acheron.floatview.protocol.refloat.RefloatConfigurationValue
import fail.acheron.floatview.protocol.refloat.RefloatConfigurationValues
import fail.acheron.floatview.protocol.vesc.ControllerRoute
import fail.acheron.floatview.protocol.vesc.VescCompatibility
import io.ktor.http.ContentType
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpStatusCode
import io.ktor.server.application.install
import io.ktor.server.cio.CIO
import io.ktor.server.engine.EmbeddedServer
import io.ktor.server.engine.embeddedServer
import io.ktor.server.request.header
import io.ktor.server.request.receive
import io.ktor.server.request.receiveText
import io.ktor.server.response.respondBytes
import io.ktor.server.response.respondText
import io.ktor.server.routing.get
import io.ktor.server.routing.post
import io.ktor.server.routing.routing
import io.ktor.server.websocket.WebSockets
import io.ktor.server.websocket.webSocket
import io.ktor.websocket.CloseReason
import io.ktor.websocket.Frame
import io.ktor.websocket.close
import io.ktor.websocket.readText
import io.ktor.websocket.send
import java.security.SecureRandom
import java.io.ByteArrayOutputStream
import java.io.File
import java.util.zip.GZIPOutputStream
import java.util.concurrent.atomic.AtomicInteger
import java.util.UUID
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.selects.select
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject

internal const val ANDROID_BACKEND_PORT = 11012
internal const val LOOPBACK_MAX_MESSAGE_BYTES = 65_536
internal const val MAX_SESSION_IMPORT_BYTES = 128 * 1024 * 1024

private class SessionDataTiming {
    val startedNs = System.nanoTime()
    var kind = "unknown"
    var sourceRows = 0
    var metadataMs = 0.0
    var configurationMs = 0.0
    var sourceMs = 0.0
    var reducerMs = 0.0
    var jsonMs = 0.0
    var gzipMs = 0.0
    var responseBytes = 0
    var compressedBytes = 0

    fun totalMs(): Double = (System.nanoTime() - startedNs) / 1_000_000.0
}

class LoopbackServer(
    private val assets: AssetManager,
    private val applicationService: ApplicationService,
    private val exportPresenter: ((SessionExportRecord) -> Boolean)?,
    private val events: EventHub,
    private val requestedPort: Int = 0,
    private val platform: String = "android",
    private val performanceDiagnostics: Boolean = false,
) {
    private val token = ByteArray(32).also(SecureRandom()::nextBytes)
        .let { Base64.encodeToString(it, Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING) }
    private val revision = AtomicInteger()
    private val sessionCatalogRevision = AtomicInteger()
    private var server: EmbeddedServer<*, *>? = null
    private var scanStopJob: Job? = null
    private var boundPort = 0
    val port: Int get() = boundPort
    val origin: String get() = "http://127.0.0.1:$port"

    fun publishSessionCatalogChanged() {
        events.publish(SessionCatalogChanged)
    }

    fun start() {
        if (server != null) return
        val html = assets.open("index.html").use { it.readBytes() }
        val started = embeddedServer(CIO, host = "127.0.0.1", port = requestedPort) {
            install(WebSockets) { maxFrameSize = LOOPBACK_MAX_MESSAGE_BYTES.toLong() }
            routing {
                get("/") {
                    call.respondBytes(html, ContentType.Text.Html)
                }
                get("/api/bootstrap") {
                    call.respondText(
                        JSONObject()
                            .put("webSocketUrl", "ws://127.0.0.1:$port/api/ws")
                            .put("sessionDataUrl", "http://127.0.0.1:$port/api/session-data")
                            .put("sessionExportUrl", "http://127.0.0.1:$port/api/session-export")
                            .put("sessionImportUrl", "http://127.0.0.1:$port/api/session-import")
                            .put("sessionToken", token)
                            .put("platform", platform)
                            .toString(),
                        ContentType.Application.Json,
                    )
                }
                post("/api/session-data") {
                    call.response.headers.append("Cache-Control", "no-store")
                    if (call.request.header("Origin") != origin || call.request.header("X-Float-View-Token") != token) {
                        call.respondText("", status = HttpStatusCode.Forbidden)
                        return@post
                    }
                    if (call.request.header("Content-Length")?.toLongOrNull()?.let { it > 16_384 } == true) {
                        call.respondText("", status = HttpStatusCode.PayloadTooLarge)
                        return@post
                    }
                    val timing = SessionDataTiming()
                    val body = call.receiveText()
                    if (body.toByteArray(Charsets.UTF_8).size > 16_384) {
                        call.respondText("", status = HttpStatusCode.PayloadTooLarge)
                        return@post
                    }
                    try {
                        val decodeStarted = System.nanoTime()
                        val requestPayload = JSONObject(body)
                        timing.jsonMs = (System.nanoTime() - decodeStarted) / 1_000_000.0
                        val sessionId = requestPayload.optString("sessionId")
                        if (sessionId.isEmpty()) {
                            call.respondText("", status = HttpStatusCode.BadRequest)
                            return@post
                        }
                        val readCancellation = SessionReadCancellation()
                        val cancellationWatcher = launch(start = CoroutineStart.UNDISPATCHED) {
                            try {
                                awaitCancellation()
                            } finally {
                                readCancellation.cancel()
                            }
                        }
                        val payload = try {
                            withContext(Dispatchers.IO) {
                                applicationService.withSessionReadCancellation(readCancellation) {
                                    val value = sessionData(requestPayload, timing)
                                    val encodeStarted = System.nanoTime()
                                    value.toString().toByteArray(Charsets.UTF_8).also {
                                        timing.jsonMs += (System.nanoTime() - encodeStarted) / 1_000_000.0
                                    }
                                }
                            }
                        } finally {
                            cancellationWatcher.cancel()
                            readCancellation.cancel()
                        }
                        if (payload.size > 4 * 1024 * 1024) {
                            call.respondText("", status = HttpStatusCode.PayloadTooLarge)
                            return@post
                        }
                        timing.responseBytes = payload.size
                        if (call.request.header("Accept-Encoding")?.contains("gzip", ignoreCase = true) == true) {
                            val compressed = ByteArrayOutputStream(payload.size)
                            val gzipStarted = System.nanoTime()
                            withContext(Dispatchers.IO) {
                                GZIPOutputStream(compressed).use { it.write(payload) }
                            }
                            timing.gzipMs = (System.nanoTime() - gzipStarted) / 1_000_000.0
                            timing.compressedBytes = compressed.size()
                            call.response.headers.append("Content-Encoding", "gzip")
                            call.respondBytes(compressed.toByteArray(), ContentType.Application.Json)
                        } else {
                            call.respondBytes(payload, ContentType.Application.Json)
                        }
                        logSessionDataTiming(timing)
                    } catch (error: CancellationException) {
                        throw error
                    } catch (_: org.json.JSONException) {
                        call.respondText("", status = HttpStatusCode.BadRequest)
                    } catch (_: IllegalArgumentException) {
                        call.respondText("", status = HttpStatusCode.BadRequest)
                    } catch (_: SessionStoreException) {
                        call.respondText("", status = HttpStatusCode.BadRequest)
                    } catch (error: Exception) {
                        Log.e("FloatViewBackend", "session-data failed (${error.javaClass.simpleName})")
                        call.respondText("", status = HttpStatusCode.InternalServerError)
                    }
                }
                post("/api/session-export") {
                    call.response.headers.append(HttpHeaders.CacheControl, "no-store")
                    if (call.request.header("Origin") != origin || call.request.header("X-Float-View-Token") != token) {
                        call.respondText("", status = HttpStatusCode.Forbidden)
                        return@post
                    }
                    if (call.request.header("Content-Length")?.toLongOrNull()?.let { it > 16_384 } == true) {
                        call.respondText("", status = HttpStatusCode.PayloadTooLarge)
                        return@post
                    }
                    try {
                        val body = call.receiveText()
                        if (body.toByteArray(Charsets.UTF_8).size > 16_384) {
                            call.respondText("", status = HttpStatusCode.PayloadTooLarge)
                            return@post
                        }
                        val payload = JSONObject(body)
                        val sessionId = payload.optString("sessionId")
                        val format = payload.optString("format")
                        if (sessionId.isEmpty() || format !in setOf("csv", "float-view-archive")) {
                            call.respondText("", status = HttpStatusCode.BadRequest)
                            return@post
                        }
                        val value = applicationService.exportSession(sessionId, format)
                        val file = File(value.filePath)
                        if (!file.isFile) {
                            call.respondText("", status = HttpStatusCode.NotFound)
                            return@post
                        }
                        call.response.headers.append(
                            HttpHeaders.ContentDisposition,
                            "attachment; filename=\"${value.fileName}\"",
                        )
                        call.respondBytes(
                            file.readBytes(),
                            if (format == "csv") ContentType.Text.CSV else ContentType.Application.OctetStream,
                        )
                    } catch (_: Exception) {
                        call.respondText("", status = HttpStatusCode.BadRequest)
                    }
                }
                post("/api/session-import") {
                    call.response.headers.append(HttpHeaders.CacheControl, "no-store")
                    if (call.request.header("Origin") != origin || call.request.header("X-Float-View-Token") != token) {
                        call.respondText("", status = HttpStatusCode.Forbidden)
                        return@post
                    }
                    if (call.request.header("Content-Length")?.toLongOrNull()?.let { it > MAX_SESSION_IMPORT_BYTES } == true) {
                        call.respondText("", status = HttpStatusCode.PayloadTooLarge)
                        return@post
                    }
                    try {
                        val body = call.receive<ByteArray>()
                        if (body.size > MAX_SESSION_IMPORT_BYTES) {
                            call.respondText("", status = HttpStatusCode.PayloadTooLarge)
                            return@post
                        }
                        val sessionId = withContext(Dispatchers.IO) {
                            applicationService.importSessionArchive(body)
                        }
                        events.publish(SessionCatalogChanged)
                        call.respondText(
                            JSONObject().put("sessionId", sessionId).toString(),
                            ContentType.Application.Json,
                        )
                    } catch (error: Exception) {
                        if (performanceDiagnostics) {
                            Log.e("FloatViewImport", "session import failed: ${error::class.simpleName}")
                        }
                        call.respondText("", status = HttpStatusCode.BadRequest)
                    }
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
                            if (event is SessionRecord || event === SessionCatalogChanged) sessionCatalogRevision.incrementAndGet()
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
                            if (text.toByteArray().size > LOOPBACK_MAX_MESSAGE_BYTES) {
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
                "location.requestPermission" -> success(locationPayload(applicationService.requestLocationPermission()))
                "location.openSettings" -> success(locationPayload(applicationService.openLocationSettings()))
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
                    val trackingMode = when (params.optString("trackingMode")) {
                        "saved" -> TrackingMode.SAVED
                        "guest" -> TrackingMode.GUEST
                        else -> return failure(id.toInt(), "invalid_request", "trackingMode must be saved or guest")
                    }
                    val active = applicationService.activeConnectionSnapshot()
                    if (
                        active != null &&
                        (active.deviceId.value != deviceId || active.state != ConnectionState.FAILED)
                    ) {
                        return failure(id.toInt(), "invalid_state", "A different board is already active")
                    }
                    if (active != null && active.trackingMode != trackingMode) {
                        return failure(id.toInt(), "invalid_state", "A retry must keep the active tracking mode")
                    }
                    applicationService.connect(DeviceId(deviceId), trackingMode)
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
                "controller.shutdown" -> {
                    if (params.length() != 0) return failure(id.toInt(), "invalid_request", "Controller shutdown does not accept parameters")
                    applicationService.shutdownController()
                    success(accepted())
                }
                "refloat.control.set" -> {
                    if (params.length() != 2) return failure(id.toInt(), "invalid_request", "A valid Refloat control and boolean are required")
                    val control = RefloatBooleanControl.entries.firstOrNull {
                        it.wireValue == params.optString("control")
                    } ?: return failure(id.toInt(), "invalid_request", "A valid Refloat control is required")
                    val enabled = params.opt("enabled") as? Boolean
                        ?: return failure(id.toInt(), "invalid_request", "enabled must be boolean")
                    success(refloatControls(applicationService.setRefloatControl(control, enabled)))
                }
                "refloat.fatalError.clear" -> {
                    if (params.length() != 0) return failure(id.toInt(), "invalid_request", "Fatal-error clear does not accept parameters")
                    success(refloatControls(applicationService.clearRefloatFatalError()))
                }
                "refloat.configuration.open" -> {
                    if (params.length() != 0) return failure(id.toInt(), "invalid_request", "Configuration open does not accept parameters")
                    success(refloatConfigurationSummary(applicationService.refloatConfigurationSummarySnapshot()))
                }
                "refloat.configuration.schema" -> {
                    if (params.keys().asSequence().any { it != "cursor" }) {
                        return failure(id.toInt(), "invalid_request", "Configuration schema parameters are invalid")
                    }
                    val page = refloatConfigurationSchemaPage(params.opt("cursor"))
                        ?: return failure(id.toInt(), "invalid_request", "Configuration schema cursor is invalid")
                    success(page)
                }
                "refloat.configuration.read" -> {
                    if (params.length() != 1 || params.optString("source") !in setOf("current", "default")) {
                        return failure(id.toInt(), "invalid_request", "Configuration source must be current or default")
                    }
                    success(refloatConfigurationValues(
                        applicationService.readRefloatConfiguration(params.optString("source") == "default"),
                    ))
                }
                "refloat.configuration.write" -> {
                    if (params.length() != 5) return failure(id.toInt(), "invalid_request", "Configuration write parameters are invalid")
                    val packageVersion = requiredString("packageVersion")
                        ?.takeIf { it.toByteArray().size <= 128 }
                        ?: return failure(id.toInt(), "invalid_request", "Package version is invalid")
                    val fingerprint = requiredString("schemaFingerprint")
                        ?.takeIf { it.matches(Regex("^[0-9a-f]{64}$")) }
                        ?: return failure(id.toInt(), "invalid_request", "Schema fingerprint is invalid")
                    val signature = params.number("wireSignature")
                        ?.takeIf { it.toDouble().isFinite() && it.toDouble() % 1.0 == 0.0 && it.toDouble() in 0.0..4_294_967_295.0 }
                        ?.toLong()
                        ?: return failure(id.toInt(), "invalid_request", "Wire signature is invalid")
                    val baseRevision = params.number("baseRevision")
                        ?.takeIf { it.toDouble().isFinite() && it.toDouble() % 1.0 == 0.0 && it.toDouble() >= 0.0 }
                        ?.toLong()
                        ?: return failure(id.toInt(), "invalid_request", "Base revision is invalid")
                    val values = params.optJSONObject("values")?.let(::parseRefloatConfigurationValues)
                        ?: return failure(id.toInt(), "invalid_request", "Complete typed configuration values are required")
                    success(refloatConfigurationValues(applicationService.writeRefloatConfiguration(
                        packageVersion, fingerprint, signature, baseRevision, values,
                    )))
                }
                "refloat.tune.list" -> {
                    if (params.keys().asSequence().any { it !in setOf("identity", "limit", "cursor") }) {
                        return failure(id.toInt(), "invalid_request", "Tune list parameters are invalid")
                    }
                    val identity = refloatConfigurationIdentity(params.opt("identity"))
                        ?: return failure(id.toInt(), "invalid_request", "Tune identity is invalid")
                    val limit = (params.opt("limit") as? Number)?.toInt()?.takeIf { it in 1..50 }
                        ?: return failure(id.toInt(), "invalid_request", "Tune page size is invalid")
                    val cursor = if (!params.has("cursor") || params.isNull("cursor")) null else {
                        (params.opt("cursor") as? String)?.takeIf { it.toByteArray().size <= 512 }
                            ?: return failure(id.toInt(), "invalid_request", "Tune cursor is invalid")
                    }
                    val page = applicationService.refloatTunes(identity, limit, cursor)
                    success(JSONObject()
                        .put("items", JSONArray(page.items.map { refloatTune(it, false) }))
                        .put("nextCursor", page.nextCursor ?: JSONObject.NULL))
                }
                "refloat.tune.get" -> {
                    val tuneId = requiredString("tuneId")?.takeIf { params.length() == 1 && it.toByteArray().size <= 128 }
                        ?: return failure(id.toInt(), "invalid_request", "Tune ID is invalid")
                    success(refloatTune(applicationService.refloatTune(tuneId), true))
                }
                "refloat.tune.create" -> {
                    val name = boundedRefloatText(params.opt("name"), 128)
                        ?: return failure(id.toInt(), "invalid_request", "Tune name is invalid")
                    if (params.length() != 5 || !params.has("description")) {
                        return failure(id.toInt(), "invalid_request", "Tune create parameters are invalid")
                    }
                    val description = if (params.isNull("description")) null else boundedRefloatText(params.opt("description"), 1_024)
                        ?: return failure(id.toInt(), "invalid_request", "Tune description is invalid")
                    val identity = refloatConfigurationIdentity(params.opt("identity"))
                        ?: return failure(id.toInt(), "invalid_request", "Tune identity is invalid")
                    val source = (params.opt("sourceRevision") as? Number)?.toDouble()
                        ?.takeIf { it >= 0 && it % 1.0 == 0.0 }?.toLong()
                        ?: return failure(id.toInt(), "invalid_request", "Tune source revision is invalid")
                    val values = params.optJSONObject("values")?.let(::parseRefloatConfigurationValues)
                        ?: return failure(id.toInt(), "invalid_request", "Tune values are invalid")
                    success(refloatTune(applicationService.createRefloatTune(
                        name, description, identity, source, values,
                    ), true))
                }
                "refloat.tune.update" -> {
                    val tuneId = requiredString("tuneId")?.takeIf { params.length() == 5 && it.toByteArray().size <= 128 }
                        ?: return failure(id.toInt(), "invalid_request", "Tune update parameters are invalid")
                    val revision = (params.opt("expectedRevision") as? Number)?.toInt()?.takeIf { it >= 1 }
                        ?: return failure(id.toInt(), "invalid_request", "Tune revision is invalid")
                    val name = boundedRefloatText(params.opt("name"), 128)
                        ?: return failure(id.toInt(), "invalid_request", "Tune name is invalid")
                    if (!params.has("description")) return failure(id.toInt(), "invalid_request", "Tune description is required")
                    val description = if (params.isNull("description")) null else boundedRefloatText(params.opt("description"), 1_024)
                        ?: return failure(id.toInt(), "invalid_request", "Tune description is invalid")
                    val values = params.optJSONObject("values")?.let(::parseRefloatConfigurationValues)
                        ?: return failure(id.toInt(), "invalid_request", "Tune values are invalid")
                    success(refloatTune(applicationService.updateRefloatTune(
                        tuneId, revision, name, description, values,
                    ), true))
                }
                "refloat.tune.delete" -> {
                    val tuneId = requiredString("tuneId")?.takeIf { params.length() == 2 }
                        ?: return failure(id.toInt(), "invalid_request", "Tune delete parameters are invalid")
                    val revision = (params.opt("expectedRevision") as? Number)?.toInt()?.takeIf { it >= 1 }
                        ?: return failure(id.toInt(), "invalid_request", "Tune revision is invalid")
                    applicationService.deleteRefloatTune(tuneId, revision)
                    success(accepted())
                }
                "refloat.tune.apply" -> {
                    val tuneId = requiredString("tuneId")?.takeIf { params.length() == 3 }
                        ?: return failure(id.toInt(), "invalid_request", "Tune apply parameters are invalid")
                    val revision = (params.opt("expectedRevision") as? Number)?.toInt()?.takeIf { it >= 1 }
                        ?: return failure(id.toInt(), "invalid_request", "Tune revision is invalid")
                    val base = (params.opt("baseRevision") as? Number)?.toDouble()
                        ?.takeIf { it >= 0 && it % 1.0 == 0.0 }?.toLong()
                        ?: return failure(id.toInt(), "invalid_request", "Tune base revision is invalid")
                    success(refloatConfigurationValues(applicationService.applyRefloatTune(tuneId, revision, base)))
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
                "savedDevice.delete" -> {
                    val savedId = requiredString("savedDeviceId")
                        ?: return failure(id.toInt(), "invalid_request", "savedDeviceId is required")
                    applicationService.deleteSavedDevice(savedId)
                    events.publish(SessionCatalogChanged)
                    success(accepted())
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
                "session.list" -> {
                    val savedDeviceId = requiredString("savedDeviceId")
                        ?: return failure(id.toInt(), "invalid_request", "savedDeviceId is required")
                    val limit = params.number("limit")?.toInt()
                        ?: return failure(id.toInt(), "invalid_request", "Session page size is required")
                    if (limit !in 1..100) return failure(id.toInt(), "invalid_request", "Session page size must be 1 through 100")
                    val board = applicationService.savedDevices().firstOrNull { it.savedDeviceId == savedDeviceId }
                        ?: return failure(id.toInt(), "not_found", "Saved board was not found")
                    val rawCursor = params.optString("cursor").takeIf { params.has("cursor") && !params.isNull("cursor") }
                    val context = rawCursor?.let(::decodeSessionCatalogCursor)
                    if (rawCursor != null && context == null) {
                        return failure(id.toInt(), "invalid_request", "Session cursor is invalid")
                    }
                    if (context != null && context.optString("deviceHistoryId") != board.deviceHistoryId) {
                        return failure(id.toInt(), "invalid_request", "Session cursor is invalid")
                    }
                    if (context != null && context.getLong("catalogRevision") != sessionCatalogRevision.get().toLong()) {
                        return failure(id.toInt(), "conflict", "Session cursor is stale")
                    }
                    val page = applicationService.sessions(
                        SessionListQuery(
                            cursor = context?.getString("innerCursor"),
                            limit = limit,
                            deviceHistoryId = board.deviceHistoryId,
                            sortField = "startedAt",
                            sortDescending = true,
                        ),
                    )
                    val nextCursor = page.nextCursor?.let { inner ->
                        encodeSessionCatalogCursor(
                            JSONObject()
                                .put("version", 1)
                                .put("query", "startedAt.desc.sessionId.desc")
                                .put("deviceHistoryId", board.deviceHistoryId)
                                .put("catalogRevision", sessionCatalogRevision.get())
                                .put("innerCursor", inner)
                                .put("lastStartedAt", page.items.lastOrNull()?.startedAt?.toString() ?: JSONObject.NULL)
                                .put("lastSessionId", page.items.lastOrNull()?.sessionId ?: JSONObject.NULL),
                        )
                    }
                    success(
                        JSONObject()
                            .put("items", JSONArray(page.items.map(::session)))
                            .put("catalogRevision", sessionCatalogRevision.get())
                            .put("nextCursor", nextCursor ?: JSONObject.NULL),
                    )
                }
                "session.get" -> {
                    val sessionId = requiredString("sessionId")
                        ?: return failure(id.toInt(), "invalid_request", "sessionId is required")
                    val summary = applicationService.session(sessionId)
                    success(
                        sessionMetadata(
                            summary,
                            applicationService.sessionExtensionFields(sessionId),
                            applicationService.sessionSnapshot(sessionId),
                            applicationService.sessionPruneMetadata(sessionId),
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
                    events.publish(SessionCatalogChanged)
                    success(accepted())
                }
                "session.delete.cancel" -> {
                    val operationId = params.optString("operationId")
                    val cancelled = applicationService.cancelPendingDelete(operationId)
                    success(operation(
                        operationId,
                        "delete",
                        if (cancelled) "cancelled" else "completed",
                        if (cancelled) operationId else null,
                        null,
                    ))
                }
                "session.prune.preview" -> {
                    val target = params.optInt("targetResolutionSeconds", 0)
                    if (target !in 1..300) return failure(id.toInt(), "invalid_request", "Prune resolution must be 1 through 300 seconds")
                    val scope = params.optString("scope", if (params.has("sessionId")) "one" else "all")
                    if (scope != "one" && scope != "all") return failure(id.toInt(), "invalid_request", "scope must be one or all")
                    val requestedSessionId = params.optString("sessionId").takeIf { it.isNotEmpty() }
                    if (scope == "one" && requestedSessionId == null) return failure(id.toInt(), "invalid_request", "sessionId is required for one-session prune preview")
                    val sessionIds = pruneSessionIds(scope, requestedSessionId)
                    val previous = applicationService.maintenanceJob()?.takeIf {
                        it.kind == "prune_preview" && it.scope == scope && it.sessionId == requestedSessionId &&
                            it.targetResolutionSeconds == target.toLong() && it.state == "pending"
                    }
                    val workIds = previous?.lastSessionId?.let { last ->
                        sessionIds.dropWhile { it != last }.drop(1)
                    } ?: sessionIds
                    var job = previous?.copy(state = "running", upperSessionId = sessionIds.lastOrNull()) ?: SessionMaintenanceJobRecord(
                        operationId = "prunePreview-${UUID.randomUUID()}",
                        kind = "prune_preview",
                        scope = scope,
                        state = "running",
                        sessionId = requestedSessionId,
                        targetResolutionSeconds = target.toLong(),
                        upperSessionId = sessionIds.lastOrNull(),
                        lastSessionId = null,
                        completedSessions = 0,
                        skippedSessions = 0,
                        retainedSamples = 0,
                        reclaimableBytes = 0,
                        error = null,
                    )
                    applicationService.saveMaintenanceJob(job)
                    var recorded = 0L
                    var stored = 0L
                    var reclaimable = 0L
                    var skipped = true
                    var skipReason: String? = null
                    workIds.forEach { sessionId ->
                        if (applicationService.maintenanceJob()?.state == "cancel_requested") return@forEach
                        val preview = applicationService.prunePreview(sessionId, target.toLong())
                        recorded += preview.recordedSampleCount
                        stored += preview.estimatedStoredSampleCount
                        reclaimable += preview.estimatedReclaimableBytes ?: 0L
                        skipped = skipped && preview.skipped
                        if (preview.skipReason != null) skipReason = preview.skipReason
                        job = job.copy(
                            lastSessionId = sessionId,
                            completedSessions = job.completedSessions + 1,
                            skippedSessions = job.skippedSessions + if (preview.skipped) 1 else 0,
                            retainedSamples = job.retainedSamples + preview.estimatedStoredSampleCount,
                            reclaimableBytes = job.reclaimableBytes + (preview.estimatedReclaimableBytes ?: 0L),
                        )
                        applicationService.saveMaintenanceJob(job)
                    }
                    job = job.copy(
                        state = if (job.state == "cancel_requested" || applicationService.maintenanceJob()?.state == "cancel_requested") "cancelled" else "completed",
                    )
                    applicationService.saveMaintenanceJob(job)
                    success(
                        JSONObject()
                            .put("operation", maintenanceJobJson(job))
                            .put("recordedSampleCount", recorded)
                            .put("estimatedStoredSampleCount", stored)
                            .put("estimatedReclaimableBytes", reclaimable)
                            .put("skipped", skipped)
                            .put("skipReason", skipReason ?: JSONObject.NULL),
                    )
                }
                "session.prune.start" -> {
                    val target = params.optInt("targetResolutionSeconds", 0)
                    if (target !in 1..300) return failure(id.toInt(), "invalid_request", "Prune resolution must be 1 through 300 seconds")
                    val scope = params.optString("scope", if (params.has("sessionId")) "one" else "all")
                    if (scope != "one" && scope != "all") return failure(id.toInt(), "invalid_request", "scope must be one or all")
                    val requestedSessionId = params.optString("sessionId").takeIf { it.isNotEmpty() }
                    if (scope == "one" && requestedSessionId == null) return failure(id.toInt(), "invalid_request", "sessionId is required for one-session prune")
                    val sessionIds = pruneSessionIds(scope, requestedSessionId)
                    val previous = applicationService.maintenanceJob()?.takeIf {
                        it.kind == "prune" && it.scope == scope && it.sessionId == requestedSessionId &&
                            it.targetResolutionSeconds == target.toLong() && it.state == "pending"
                    }
                    val workIds = previous?.lastSessionId?.let { last ->
                        sessionIds.dropWhile { it != last }.drop(1)
                    } ?: sessionIds
                    var job = previous?.copy(state = "running", upperSessionId = sessionIds.lastOrNull()) ?: SessionMaintenanceJobRecord(
                        operationId = "prune-${UUID.randomUUID()}",
                        kind = "prune",
                        scope = scope,
                        state = "running",
                        sessionId = requestedSessionId,
                        targetResolutionSeconds = target.toLong(),
                        upperSessionId = sessionIds.lastOrNull(),
                        lastSessionId = null,
                        completedSessions = 0,
                        skippedSessions = 0,
                        retainedSamples = 0,
                        reclaimableBytes = 0,
                        error = null,
                    )
                    applicationService.saveMaintenanceJob(job)
                    try {
                        workIds.forEach { sessionId ->
                            if (applicationService.maintenanceJob()?.state == "cancel_requested") return@forEach
                            val result = applicationService.pruneSession(sessionId, target.toLong())
                            job = job.copy(
                                lastSessionId = sessionId,
                                completedSessions = job.completedSessions + 1,
                                skippedSessions = job.skippedSessions + if (result.skipped) 1 else 0,
                                retainedSamples = job.retainedSamples + result.estimatedStoredSampleCount,
                                reclaimableBytes = job.reclaimableBytes + (result.estimatedReclaimableBytes ?: 0L),
                            )
                            applicationService.saveMaintenanceJob(job)
                        }
                    } catch (error: Exception) {
                        applicationService.saveMaintenanceJob(job.copy(state = "failed", error = error.message?.take(256)))
                        throw error
                    }
                    job = job.copy(
                        state = if (job.state == "cancel_requested" || applicationService.maintenanceJob()?.state == "cancel_requested") "cancelled" else "completed",
                    )
                    applicationService.saveMaintenanceJob(job)
                    events.publish(SessionCatalogChanged)
                    success(maintenanceJobJson(job))
                }
                "session.prune.cancel" -> {
                    val job = applicationService.cancelMaintenanceJob(params.optString("operationId"))
                    success(job?.let(::maintenanceJobJson) ?: operation(params.optString("operationId"), "prune", "cancelled", null, null))
                }
                else -> failure(id.toInt(), "invalid_request", "Unknown method")
            }
        } catch (error: RefloatMutationException) {
            failure(
                id.toInt(), if (error.unavailable) "unavailable" else "invalid_state",
                if (error.unavailable) "Refloat control confirmation was unavailable" else "Refloat control is not permitted in the current state",
            )
        } catch (error: fail.acheron.floatview.backend.RefloatConfigurationServiceException) {
            when (error.reason) {
                fail.acheron.floatview.backend.RefloatConfigurationServiceError.CONFLICT ->
                    failure(id.toInt(), "conflict", "Configuration changed or verification did not match")
                fail.acheron.floatview.backend.RefloatConfigurationServiceError.UNAVAILABLE ->
                    failure(id.toInt(), "unavailable", "Configuration request did not complete")
                else -> failure(id.toInt(), "invalid_state", "Configuration is not available in the current state")
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
                is ProductStoreException.RefloatConfigurationRevisionConflict,
                -> failure(id.toInt(), "conflict", "Revision conflict")
                ProductStoreException.SavedDeviceNotFound ->
                    failure(id.toInt(), "not_found", "Saved device was not found")
                ProductStoreException.RefloatConfigurationNotFound ->
                    failure(id.toInt(), "not_found", "Tune was not found")
                ProductStoreException.InvalidProfilePatch,
                ProductStoreException.InvalidSavedDevicePatch,
                ProductStoreException.InvalidSettingsPatch,
                ProductStoreException.InvalidRefloatConfiguration,
                -> failure(id.toInt(), "invalid_request", "Patch is invalid")
                ProductStoreException.SavedDeviceActive ->
                    failure(id.toInt(), "invalid_state", "Disconnect the board before forgetting it")
            }
        } catch (error: SessionStoreException) {
            when (error) {
                SessionStoreException.SessionNotFound ->
                    failure(id.toInt(), "not_found", "Session was not found")
                SessionStoreException.SessionActive ->
                    failure(id.toInt(), "invalid_state", "Session is active")
                SessionStoreException.MaintenanceInProgress ->
                    failure(id.toInt(), "unavailable", "Session maintenance is in progress")
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
                            "accessMode",
                            when (VescCompatibility.accessMode(firmware)) {
                                fail.acheron.floatview.protocol.vesc.VescAccessMode.READ_ONLY -> "readOnly"
                                fail.acheron.floatview.protocol.vesc.VescAccessMode.READ_WRITE -> "readWrite"
                                null -> "readOnly"
                            },
                        )
                        .put(
                            "supported",
                            RefloatCompatibility.entry(packageVersion) != null &&
                                VescCompatibility.supports(firmware),
                        )
                } else {
                    JSONObject.NULL
                },
            )
            activePayload.put(
                "refloatConfiguration",
                if (packageVersion == null) JSONObject.NULL
                else refloatConfigurationSummary(applicationService.refloatConfigurationSummarySnapshot()),
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
            .put("sessionCatalogRevision", sessionCatalogRevision.get())
            .put("settings", runCatching { settings(applicationService.settings()) }.getOrDefault(JSONObject()))
            .put("location", locationPayload(applicationService.locationState()))
            .put("sessionOperation", runCatching { applicationService.maintenanceJob() }
                .getOrNull()?.let(::maintenanceJobJson) ?: JSONObject.NULL)
    }

    private fun bluetooth(value: BluetoothState, scanning: Boolean = false) = JSONObject()
        .put("radio", value.radio.name.lowercase())
        .put("scanPermission", value.scanPermission.name.lowercase())
        .put("connectPermission", value.connectPermission.name.lowercase())
        .put("scanning", scanning)

    private fun refloatConfigurationSummary(value: RefloatConfigurationSummary) = JSONObject()
        .put("availability", value.availability.name.lowercase())
        .put("writable", value.writable)
        .put("packageVersion", value.packageVersion ?: JSONObject.NULL)
        .put("schemaFingerprint", value.schemaFingerprint ?: JSONObject.NULL)
        .put("wireSignature", value.wireSignature ?: JSONObject.NULL)
        .put("currentLoaded", value.currentLoaded)
        .put("defaultLoaded", value.defaultLoaded)
        .put("pendingOperation", when (value.pendingOperation) {
            fail.acheron.floatview.protocol.refloat.RefloatConfigurationOperation.SCHEMA -> "schema"
            fail.acheron.floatview.protocol.refloat.RefloatConfigurationOperation.READ_CURRENT -> "readCurrent"
            fail.acheron.floatview.protocol.refloat.RefloatConfigurationOperation.READ_DEFAULT -> "readDefault"
            fail.acheron.floatview.protocol.refloat.RefloatConfigurationOperation.WRITE -> "write"
            fail.acheron.floatview.protocol.refloat.RefloatConfigurationOperation.APPLY_TUNE -> "applyTune"
            null -> JSONObject.NULL
        })

    private fun refloatConfigurationIdentity(): JSONObject? {
        val summary = applicationService.refloatConfigurationSummarySnapshot()
        val version = summary.packageVersion ?: return null
        val fingerprint = summary.schemaFingerprint ?: return null
        val signature = summary.wireSignature ?: return null
        return JSONObject()
            .put("packageName", "refloat")
            .put("packageVersion", version)
            .put("schemaFingerprint", fingerprint)
            .put("wireSignature", signature)
    }

    private fun refloatConfigurationIdentity(raw: Any?): RefloatConfigurationIdentityRecord? {
        val value = raw as? JSONObject ?: return null
        if (value.length() != 4 || value.optString("packageName") != "refloat") return null
        val version = value.optString("packageVersion").takeIf { it.isNotEmpty() && it.toByteArray().size <= 128 }
            ?: return null
        val fingerprint = value.optString("schemaFingerprint").takeIf { it.matches(Regex("^[0-9a-f]{64}$")) }
            ?: return null
        val signature = (value.opt("wireSignature") as? Number)?.toDouble()
            ?.takeIf { it.isFinite() && it % 1.0 == 0.0 && it in 0.0..4_294_967_295.0 }?.toLong()
            ?: return null
        return RefloatConfigurationIdentityRecord("refloat", version, fingerprint, signature)
    }

    private fun boundedRefloatText(raw: Any?, maximumBytes: Int): String? =
        (raw as? String)?.trim()?.takeIf { it.isNotEmpty() && it.toByteArray().size <= maximumBytes }

    private fun refloatTune(record: RefloatConfigurationRecord, includeValues: Boolean) = JSONObject()
        .put("tuneId", record.configurationId)
        .put("revision", record.revision)
        .put("name", record.name)
        .put("description", record.description ?: JSONObject.NULL)
        .put("identity", JSONObject()
            .put("packageName", record.identity.packageName)
            .put("packageVersion", record.identity.packageVersion)
            .put("schemaFingerprint", record.identity.schemaFingerprint)
            .put("wireSignature", record.identity.wireSignature))
        .put("createdAt", record.createdAt.toString())
        .put("updatedAt", record.updatedAt.toString())
        .also { result ->
            if (includeValues) {
                val values = runCatching { JSONObject(record.valuesJson) }.getOrDefault(JSONObject())
                result.put("values", values)
            }
        }

    private fun refloatConfigurationSchemaPage(rawCursor: Any?): JSONObject? {
        val schema = applicationService.refloatConfigurationSchemaSnapshot() ?: return null
        val identity = refloatConfigurationIdentity() ?: return null
        val pageIndex = when (rawCursor) {
            null, JSONObject.NULL -> 0
            is String -> {
                if (rawCursor.toByteArray().size > 512) return null
                runCatching {
                    JSONObject(String(Base64.decode(rawCursor, Base64.DEFAULT), Charsets.UTF_8))
                }.getOrNull()?.takeIf { it.optString("fingerprint") == schema.fingerprint }
                    ?.optInt("page", -1)?.takeIf { it >= 0 } ?: return null
            }
            else -> return null
        }
        val byName = schema.fields.associateBy { it.name }
        val pages = buildList {
            schema.groups.forEach { group ->
                group.subgroups.forEach { subgroup ->
                    subgroup.fields.mapNotNull(byName::get).chunked(32).forEach { fields ->
                        add(Triple(group.name, subgroup.name, fields))
                    }
                }
            }
        }
        val page = pages.getOrNull(pageIndex) ?: return null
        val nextCursor = if (pageIndex + 1 < pages.size) {
            Base64.encodeToString(
                JSONObject().put("fingerprint", schema.fingerprint).put("page", pageIndex + 1)
                    .toString().toByteArray(),
                Base64.NO_WRAP,
            )
        } else JSONObject.NULL
        return JSONObject()
            .put("identity", identity)
            .put("group", page.first)
            .put("subgroup", page.second)
            .put("fields", JSONArray(page.third.map(::refloatConfigurationField)))
            .put("nextCursor", nextCursor)
    }

    private fun refloatConfigurationField(field: RefloatConfigurationField) = JSONObject()
        .put("name", field.name)
        .put("label", field.longName)
        .put("type", when (field.type) {
            RefloatConfigurationFieldType.DOUBLE -> "double"
            RefloatConfigurationFieldType.INTEGER -> "integer"
            RefloatConfigurationFieldType.STRING -> "string"
            RefloatConfigurationFieldType.ENUMERATION -> "enum"
            RefloatConfigurationFieldType.BOOLEAN -> "boolean"
            RefloatConfigurationFieldType.BITFIELD -> "bitfield"
        })
        .put("defaultValue", refloatConfigurationValue(field.defaultValue))
        .put("minimum", field.minimum ?: JSONObject.NULL)
        .put("maximum", field.maximum ?: JSONObject.NULL)
        .put("step", field.step ?: JSONObject.NULL)
        .put("precision", field.precision ?: JSONObject.NULL)
        .put("suffix", field.suffix)
        .put("choices", JSONArray(field.choices))
        .put("description", field.description)

    private fun refloatConfigurationValue(value: RefloatConfigurationValue) = when (value) {
        is RefloatConfigurationValue.Decimal -> JSONObject().put("type", "double").put("value", value.value)
        is RefloatConfigurationValue.Integer -> JSONObject().put("type", "integer").put("value", value.value)
        is RefloatConfigurationValue.BooleanValue -> JSONObject().put("type", "boolean").put("value", value.value)
        is RefloatConfigurationValue.StringValue -> JSONObject().put("type", "string").put("value", value.value)
    }

    private fun refloatConfigurationValues(value: RefloatConfigurationValues) = JSONObject()
        .put("identity", refloatConfigurationIdentity() ?: JSONObject())
        .put("revision", value.revision)
        .put("source", value.source)
        .put("values", JSONObject().also { result ->
            value.values.forEach { (name, fieldValue) -> result.put(name, refloatConfigurationValue(fieldValue)) }
        })

    private fun parseRefloatConfigurationValues(raw: JSONObject): Map<String, RefloatConfigurationValue>? {
        if (raw.length() > RefloatConfigurationLimits.FIELDS || raw.toString().toByteArray().size > 48 * 1024) return null
        val result = linkedMapOf<String, RefloatConfigurationValue>()
        raw.keys().forEach { name ->
            if (name.isEmpty() || name.toByteArray().size > RefloatConfigurationLimits.NAME_BYTES) return null
            val typed = raw.optJSONObject(name)?.takeIf { it.length() == 2 && it.has("type") && it.has("value") }
                ?: return null
            result[name] = when (typed.optString("type")) {
                "double" -> {
                    val number = typed.opt("value") as? Number ?: return null
                    val value = number.toDouble().takeIf(Double::isFinite) ?: return null
                    RefloatConfigurationValue.Decimal(value)
                }
                "integer" -> {
                    val number = typed.opt("value") as? Number ?: return null
                    val value = number.toDouble()
                    if (!value.isFinite() || value % 1.0 != 0.0 || kotlin.math.abs(value) > 9_007_199_254_740_991.0) return null
                    RefloatConfigurationValue.Integer(value.toLong())
                }
                "boolean" -> RefloatConfigurationValue.BooleanValue(
                    typed.opt("value") as? Boolean ?: return null,
                )
                "string" -> {
                    val value = typed.opt("value") as? String ?: return null
                    if (value.toByteArray().size > RefloatConfigurationLimits.STRING_VALUE_BYTES) return null
                    RefloatConfigurationValue.StringValue(value)
                }
                else -> return null
            }
        }
        return result
    }

    private fun device(value: DiscoveredDevice) = JSONObject()
        .put("deviceId", value.id.value)
        .put("name", value.advertisedName ?: "")
        .put("state", "discovered")
        .put("rssi", value.rssi ?: JSONObject.NULL)
        .put("lastSeenAt", value.lastSeenAt.toString())
        .put("savedDeviceId", value.savedDeviceId ?: JSONObject.NULL)

    private fun connection(value: ActiveConnectionSnapshot) = JSONObject()
        .put("deviceId", value.deviceId.value)
        .put("deviceKind", value.deviceKind?.name?.lowercase() ?: JSONObject.NULL)
        .put("trackingMode", if (value.trackingMode == TrackingMode.SAVED) "saved" else "guest")
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
        .put("bms", bmsState(value.bms))
        .put("refloatControls", value.refloatControls?.let(::refloatControls) ?: JSONObject.NULL)
        .put("controllerActions", controllerActions(value.controllerActions))
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
        .put("latestLocation", value.latestLocation?.let(::liveLocation) ?: JSONObject.NULL)
        .put(
            "currentLimits",
            value.currentLimits?.let { limits ->
                JSONObject()
                    .put("motorCurrentMaxA", limits.motorCurrentMaxA)
                    .put("motorCurrentMinA", limits.motorCurrentMinA)
                    .put("batteryCurrentMaxA", limits.batteryCurrentMaxA)
                    .put("batteryCurrentMinA", limits.batteryCurrentMinA)
                    .put("batterySeriesCells", limits.batterySeriesCells ?: JSONObject.NULL)
            } ?: JSONObject.NULL,
        )

    private fun refloatControls(value: RefloatControls) = JSONObject()
        .put("status", value.status.wireValue)
        .put("writable", value.writable)
        .put("packageEnabled", value.packageEnabled ?: JSONObject.NULL)
        .put("flywheelEnabled", value.flywheelEnabled ?: JSONObject.NULL)
        .put("lightingAvailable", value.lightingAvailable)
        .put("ledsEnabled", value.ledsEnabled ?: JSONObject.NULL)
        .put("headlightsEnabled", value.headlightsEnabled ?: JSONObject.NULL)
        .put("fatalErrorClearAvailable", value.fatalErrorClearAvailable)
        .put("fatalErrorLatched", value.fatalErrorLatched ?: JSONObject.NULL)
        .put("fatalErrorActive", value.fatalErrorActive ?: JSONObject.NULL)
        .put("fatalErrorClearable", value.fatalErrorClearable ?: JSONObject.NULL)
        .put("pendingAction", value.pendingAction?.wireValue ?: JSONObject.NULL)

    private fun controllerActions(value: ControllerActions) = JSONObject()
        .put("shutdownAvailable", value.shutdownAvailable)
        .put("pendingAction", value.pendingAction?.wireValue ?: JSONObject.NULL)

    private fun bmsConnection(value: fail.acheron.floatview.backend.BmsConnection) = when (value) {
        fail.acheron.floatview.backend.BmsConnection.Direct -> JSONObject().put("kind", "direct")
        is fail.acheron.floatview.backend.BmsConnection.Can -> JSONObject()
            .put("kind", "can")
            .put("canId", value.canId)
    }

    private fun bmsState(value: fail.acheron.floatview.backend.BmsState) = JSONObject()
        .put("status", value.status.wireValue)
        .put("connection", value.connection?.let(::bmsConnection) ?: JSONObject.NULL)
        .put(
            "candidates",
            JSONArray(value.candidates.take(16).map { candidate ->
                JSONObject()
                    .put("connection", bmsConnection(candidate.connection))
                    .put("hardwareName", candidate.hardwareName ?: JSONObject.NULL)
                    .put("vescVersion", candidate.vescVersion)
            }),
        )
        .put("latestSnapshot", value.latestSnapshot?.let(::bmsSnapshot) ?: JSONObject.NULL)

    private fun bmsSnapshot(value: fail.acheron.floatview.backend.BmsSnapshot) = JSONObject()
        .put("observedAt", value.observedAt.toString())
        .put("totalVoltageV", value.totalVoltageV)
        .put("chargeVoltageV", value.chargeVoltageV)
        .put("inputCurrentA", value.inputCurrentA)
        .put("inputIcCurrentA", value.inputIcCurrentA)
        .put("sessionAmpHoursAh", value.sessionAmpHoursAh)
        .put("sessionWattHoursWh", value.sessionWattHoursWh)
        .put("cellVoltagesV", JSONArray(value.cellVoltagesV.take(50)))
        .put("balancing", JSONArray(value.balancing.take(50)))
        .put("temperaturesC", JSONArray(value.temperaturesC.take(50)))
        .put("icTemperatureC", value.icTemperatureC)
        .put("humiditySensorTemperatureC", value.humiditySensorTemperatureC)
        .put("humidityPercent", value.humidityPercent)
        .put("highestCellTemperatureC", value.highestCellTemperatureC)
        .put("stateOfCharge", value.stateOfCharge)
        .put("stateOfHealth", value.stateOfHealth)
        .put("reportedCanId", value.reportedCanId)
        .put("lifetimeChargeAmpHoursAh", value.lifetimeChargeAmpHoursAh)
        .put("lifetimeChargeWattHoursWh", value.lifetimeChargeWattHoursWh)
        .put("lifetimeDischargeAmpHoursAh", value.lifetimeDischargeAmpHoursAh)
        .put("lifetimeDischargeWattHoursWh", value.lifetimeDischargeWattHoursWh)
        .put("pressurePa", value.pressurePa)
        .put("dataVersion", value.dataVersion ?: JSONObject.NULL)
        .put("status", value.status?.takeIf { it.toByteArray().size <= 40 } ?: JSONObject.NULL)

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
        .put(
            "summary",
            JSONObject()
                .put("sessionCount", value.summary.sessionCount)
                .put("totalRidingElapsedMs", value.summary.totalRidingElapsedMs)
                .put("totalDistanceM", value.summary.totalDistanceM ?: JSONObject.NULL)
                .put("mostRecentSessionAt", value.summary.mostRecentSessionAt?.toString() ?: JSONObject.NULL),
        )

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

    private fun sessionMetadata(
        value: SessionSummaryRecord,
        extensions: List<String>,
        snapshot: SessionPublishedSnapshotRecord,
        prune: SessionPruneMetadataRecord,
    ) = JSONObject()
        .put("summary", session(value))
        .put("fields", JSONArray((NUMERIC_FIELDS.map { field ->
            JSONObject().put("name", field).put("source", "stable")
        } + extensions.map { field ->
            JSONObject().put("name", field).put("source", "extension")
        })))
        .put("eventCount", value.eventCount)
        .put("recordedSampleCount", snapshot.recordedSampleCount)
        .put("storedSampleCount", snapshot.storedSampleCount)
        .put("location", JSONObject()
            .put("available", snapshot.recordedLocationCount > 0)
            .put("count", snapshot.recordedLocationCount)
            .put("lastSequence", snapshot.committedLocationSequence)
            .put("lastElapsedMs", snapshot.committedLocationElapsedMs)
            .put("status", snapshot.locationStatus))
        .put("prune", JSONObject()
            .put("targetResolutionSeconds", prune.targetResolutionSeconds ?: JSONObject.NULL)
            .put("recordedSampleCount", prune.recordedSampleCount)
            .put("storedSampleCount", prune.storedSampleCount)
            .put("policy", prune.policy ?: JSONObject.NULL))
        .put("snapshot", JSONObject()
            .put("historyRevision", snapshot.historyRevision)
            .put("committedSampleSequence", snapshot.committedSampleSequence)
            .put("committedEventSequence", snapshot.committedEventSequence)
            .put("committedElapsedMs", snapshot.committedElapsedMs)
            .put("committedLocationSequence", snapshot.committedLocationSequence)
            .put("committedLocationElapsedMs", snapshot.committedLocationElapsedMs))

    private fun sessionData(params: JSONObject, timing: SessionDataTiming): JSONObject {
        val kind = params.getString("kind")
        timing.kind = kind
        val sessionId = params.getString("sessionId")
        val range = params.getJSONObject("range")
        val start = range.getLong("startElapsedMs")
        val end = range.getLong("endElapsedMs")
        val snapshot = params.getJSONObject("snapshot")
        val metadataStarted = System.nanoTime()
        val published = applicationService.sessionSnapshot(sessionId)
        require(snapshot.getLong("historyRevision") == published.historyRevision)
        require(snapshot.getLong("committedSampleSequence") == published.committedSampleSequence)
        require(snapshot.getLong("committedEventSequence") == published.committedEventSequence)
        require(snapshot.getLong("committedElapsedMs") == published.committedElapsedMs)
        require(snapshot.getLong("committedLocationSequence") == published.committedLocationSequence)
        require(snapshot.getLong("committedLocationElapsedMs") == published.committedLocationElapsedMs)
        val echoedSnapshot = JSONObject()
            .put("historyRevision", snapshot.getLong("historyRevision"))
            .put("committedSampleSequence", snapshot.getLong("committedSampleSequence"))
            .put("committedEventSequence", snapshot.getLong("committedEventSequence"))
            .put("committedElapsedMs", snapshot.getLong("committedElapsedMs"))
            .put("committedLocationSequence", snapshot.getLong("committedLocationSequence"))
            .put("committedLocationElapsedMs", snapshot.getLong("committedLocationElapsedMs"))
        val fields = params.optJSONArray("fields") ?: JSONArray()
        require(fields.length() in 1..16 || kind == "events" || kind == "locations")
        val fieldNames = (0 until fields.length()).map { fields.getString(it) }
        val extensionFields = applicationService.sessionExtensionFields(sessionId).toSet()
        timing.metadataMs = (System.nanoTime() - metadataStarted) / 1_000_000.0
        if (kind == "series" || kind == "samples") {
            val descriptors = JSONArray(fieldNames.map { JSONObject().put("name", it).put("source", if (it in extensionFields) "extension" else "stable") })
            val base = JSONObject()
                .put("kind", kind)
                .put("sessionId", sessionId)
                .put("snapshot", echoedSnapshot)
                .put("range", range)
                .put("fields", descriptors)
            if (kind == "series") {
                val buckets = params.optInt("buckets", 1).coerceIn(1, 1_024)
                val gap = params.optJSONObject("gapPolicy") ?: throw SessionStoreException.InvalidRange
                val threshold = if (gap.isNull("thresholdSeconds")) null else gap.getDouble("thresholdSeconds")
                if (threshold != null && (!threshold.isFinite() || threshold < 0.1 || threshold > 3_600)) {
                    throw SessionStoreException.InvalidRange
                }
                val configurationStarted = System.nanoTime()
                val historyConfiguration = applicationService.sessionHistoryConfiguration(sessionId)
                timing.configurationMs = (System.nanoTime() - configurationStarted) / 1_000_000.0
                val streamed = applicationService.reduceSessionSeries(
                    sessionId = sessionId,
                    startElapsedMs = start,
                    endElapsedMs = end,
                    fields = fieldNames,
                    buckets = buckets,
                    gapPolicy = SessionGapPolicyRecord(threshold),
                    expectedPersistenceMs = historyConfiguration.expectedIdlePersistenceMs,
                    expectedRidingPersistenceMs = historyConfiguration.expectedRidingPersistenceMs,
                    expectedIdlePersistenceMs = historyConfiguration.expectedIdlePersistenceMs,
                    retainedResolutionSeconds = historyConfiguration.retainedResolutionSeconds,
                    publishedSampleSequence = snapshot.getLong("committedSampleSequence"),
                )
                // The reducer is now fed directly from the SQLite cursor. The
                // native reader reports both portions without a second full-row
                // pass or retained source array.
                timing.sourceMs = streamed.sourceMs
                timing.reducerMs = streamed.reducerMs
                timing.sourceRows = streamed.sourceRows
                val columns = JSONObject()
                fieldNames.forEach { field ->
                    val column = streamed.columns.getValue(field)
                    columns.put(field, JSONObject()
                        .put("elapsedMs", JSONArray(column.elapsedMs))
                        .put("sequence", JSONArray(column.sequence))
                        .put("values", JSONArray(column.values.map { it ?: JSONObject.NULL }))
                        .put("generations", JSONArray(column.generations)))
                }
                return base.put("gapPolicy", JSONObject().put("thresholdSeconds", threshold ?: JSONObject.NULL)).put("columns", columns)
            }
            val seed = SessionHistoryCursor(
                sessionId,
                published,
                SessionElapsedRange(start, end),
                start,
                0,
            )
            val decodedCursor = params.optString("cursor").takeIf { it.isNotEmpty() && !params.isNull("cursor") }?.let { raw ->
                SessionHistoryReducer.decodeCursor(raw).also { cursor ->
                    if (cursor.sessionId != sessionId || cursor.range != seed.range ||
                        cursor.snapshot.historyRevision != published.historyRevision ||
                        cursor.snapshot.committedSampleSequence != published.committedSampleSequence ||
                        cursor.snapshot.committedEventSequence != published.committedEventSequence ||
                        cursor.snapshot.committedElapsedMs != published.committedElapsedMs
                    ) throw SessionStoreException.InvalidCursor
                }
            }
            val source = applicationService.querySessionSeriesSource(
                sessionId, start, end, fieldNames, 200_000,
                snapshot.getLong("committedSampleSequence"),
            )
            val page = SessionHistoryReducer.page(
                source,
                params.optInt("limit", 2_000).coerceIn(1, 2_000),
                decodedCursor,
                seed,
            )
            val points = page.first
            val nextCursor = page.second?.let(SessionHistoryReducer::encodeCursor)
            val values = JSONObject()
            fieldNames.forEach { field -> values.put(field, JSONArray(points.map { it.values[field] ?: JSONObject.NULL })) }
            return base.put("mode", "window")
                .put("elapsedMs", JSONArray(points.map { it.elapsedMs }))
                .put("sequence", JSONArray(points.map { it.sequence }))
                .put("values", values)
                .put("nextCursor", nextCursor ?: JSONObject.NULL)
        }
        if (kind == "locations") {
            val page = applicationService.querySessionLocations(
                sessionId = sessionId,
                startElapsedMs = start,
                endElapsedMs = end,
                limit = params.optInt("limit", 1).coerceIn(1, 2_000),
                cursor = params.optString("cursor").takeIf { it.isNotEmpty() && !params.isNull("cursor") },
                publishedLocationSequence = published.committedLocationSequence,
            )
            return JSONObject()
                .put("kind", kind)
                .put("sessionId", sessionId)
                .put("snapshot", echoedSnapshot)
                .put("range", range)
                .put("points", JSONArray(page.points.map { point ->
                    JSONObject().put("locationSequence", point.locationSequence)
                        .put("elapsedMs", point.elapsedMs)
                        .put("latitude", point.latitude)
                        .put("longitude", point.longitude)
                        .put("accuracyM", point.accuracyM ?: JSONObject.NULL)
                        .put("observedAt", point.observedAt.toString())
                }))
                .put("gaps", JSONArray(page.gaps.map { gap ->
                    JSONObject().put("startElapsedMs", gap.startElapsedMs)
                        .put("endElapsedMs", gap.endElapsedMs)
                        .put("reason", gap.reason)
                }))
                .put("nextCursor", page.nextCursor ?: JSONObject.NULL)
                .put("locationBoundary", JSONObject()
                    .put("lastSequence", page.snapshot.committedLocationSequence)
                    .put("lastElapsedMs", page.snapshot.committedLocationElapsedMs))
        }
        require(kind == "events")
        val seed = SessionHistoryCursor(
            sessionId,
            published,
            SessionElapsedRange(start, end),
            start,
            0,
        )
        val decodedCursor = params.optString("cursor").takeIf { it.isNotEmpty() && !params.isNull("cursor") }?.let { raw ->
            SessionHistoryReducer.decodeCursor(raw).also { cursor ->
                if (cursor.sessionId != sessionId || cursor.range != seed.range ||
                    cursor.snapshot.historyRevision != published.historyRevision ||
                    cursor.snapshot.committedSampleSequence != published.committedSampleSequence ||
                    cursor.snapshot.committedEventSequence != published.committedEventSequence ||
                    cursor.snapshot.committedElapsedMs != published.committedElapsedMs
                ) throw SessionStoreException.InvalidCursor
            }
        }
        val allEvents = applicationService.sessionEvents(
            sessionId, start, end, snapshot.getLong("committedEventSequence"),
        ).sortedWith(compareBy<SessionEventRecord> { it.elapsedMs }.thenBy { it.eventId.toLongOrNull() ?: 0L })
        val pageEvents = allEvents.filter { event ->
            val cursor = decodedCursor ?: return@filter true
            val sequence = event.eventId.toLongOrNull() ?: 0L
            event.elapsedMs > cursor.elapsedMs || (event.elapsedMs == cursor.elapsedMs && sequence > cursor.sequence)
        }
        val events = pageEvents.take(params.optInt("limit", 500).coerceIn(1, 500))
        val nextCursor = if (events.isNotEmpty() && events.size < pageEvents.size) {
            val last = events.last()
            SessionHistoryReducer.encodeCursor(seed.copy(
                elapsedMs = last.elapsedMs,
                sequence = last.eventId.toLongOrNull() ?: 0L,
            ))
        } else null
        return JSONObject()
            .put("kind", "events")
            .put("sessionId", sessionId)
            .put("snapshot", echoedSnapshot)
            .put("range", range)
            .put("page", JSONObject()
                .put("elapsedMs", JSONArray(events.map { it.elapsedMs }))
                .put("sequence", JSONArray(events.map { it.eventId.toLongOrNull() ?: 0 }))
                .put("sourceSampleSequence", JSONArray(events.map { it.sourceSampleSequence ?: 0 }))
                .put("kinds", JSONArray(events.map { it.kind }))
                .put("values", JSONArray(events.map { JSONObject(it.detailsJson).optString("value", "") })))
            .put("nextCursor", nextCursor ?: JSONObject.NULL)
    }

    private fun logSessionDataTiming(timing: SessionDataTiming) {
        if (!performanceDiagnostics) return
        Log.d(
            "FloatViewPerformance",
            "session-data kind=${timing.kind} rows=${timing.sourceRows} " +
                "metadata_ms=${timing.metadataMs} config_ms=${timing.configurationMs} " +
                "source_ms=${timing.sourceMs} reducer_ms=${timing.reducerMs} " +
                "json_ms=${timing.jsonMs} gzip_ms=${timing.gzipMs} " +
                "response_bytes=${timing.responseBytes} compressed_bytes=${timing.compressedBytes} " +
                "total_ms=${timing.totalMs()}",
        )
    }

    private fun operation(
        operationId: String,
        kind: String,
        status: String,
        sessionId: String?,
        target: Int?,
        completedSessions: Int = if (status == "completed") 1 else 0,
        eligibleSessions: Int = 1,
    ) = JSONObject()
        .put("operationId", operationId)
        .put("kind", kind)
        .put("status", status)
        .put("sessionId", sessionId ?: JSONObject.NULL)
        .put("scope", if (sessionId == null) "all" else "one")
        .put("targetResolutionSeconds", target ?: JSONObject.NULL)
        .put("completedSessions", completedSessions)
        .put("eligibleSessions", eligibleSessions)
        .put("error", JSONObject.NULL)

    private fun maintenanceJobJson(job: SessionMaintenanceJobRecord) = JSONObject()
        .put("operationId", job.operationId)
        .put("kind", if (job.kind == "prune_preview") "prunePreview" else "prune")
        .put("status", if (job.state == "cancel_requested") "cancelling" else job.state)
        .put("sessionId", job.sessionId ?: JSONObject.NULL)
        .put("scope", job.scope)
        .put("targetResolutionSeconds", job.targetResolutionSeconds)
        .put("completedSessions", job.completedSessions)
        .put("eligibleSessions", if (job.upperSessionId == null) JSONObject.NULL else job.completedSessions)
        .put("error", job.error ?: JSONObject.NULL)

    private fun pruneSessionIds(scope: String, sessionId: String?): List<String> {
        if (scope == "one") return listOf(requireNotNull(sessionId))
        var cursor: String? = null
        val ids = mutableListOf<String>()
        do {
            val page = applicationService.sessions(
                SessionListQuery(
                    cursor = cursor,
                    limit = 100,
                    state = "completed",
                    integrity = "complete",
                    sortField = "startedAt",
                    sortDescending = true,
                ),
            )
            ids += page.items.map { it.sessionId }
            cursor = page.nextCursor
        } while (cursor != null)
        return ids
    }

    private fun seriesPoint(value: SessionSeriesPoint): JSONObject {
        val fields = JSONObject()
        value.values.forEach { (key, field) -> fields.put(key, field ?: JSONObject.NULL) }
        return JSONObject().put("elapsedMs", value.elapsedMs).put("values", fields)
    }

    private fun settings(value: SettingsRecord) = JSONObject()
        .put("revision", value.revision)
        .put("idleProbeIntervalMs", value.idleProbeIntervalMs)
        .put("idlePersistIntervalMs", value.idlePersistIntervalMs)
        .put("ridingPersistIntervalMs", value.ridingPersistIntervalMs)
        .put("postRideTailMs", value.postRideTailMs)
        .put("reconnectDurationMs", value.reconnectDurationMs)
        .put("minimumSessionDistance", value.minimumSessionDistance)
        .put("themePreference", value.themePreference.wireValue)
        .put("unitPreference", value.unitPreference.wireValue)
        .put("historyGapThresholdSeconds", value.historyGapThresholdSeconds ?: JSONObject.NULL)
        .put("locationRecordingEnabled", value.locationRecordingEnabled)
        .put("includeGpsLocationWhenExporting", value.includeGpsLocationWhenExporting)

    private fun locationPayload(value: LocationProviderState): JSONObject {
        val enabled = runCatching { applicationService.settings().locationRecordingEnabled }.getOrDefault(false)
        if (!enabled) {
            return JSONObject()
                .put("status", "disabled")
                .put("permissionAction", "requestPermission")
                .put("backgroundCapture", "unavailable")
                .put("reason", JSONObject.NULL)
        }
        val status = when (value.authorization) {
            LocationProviderAuthorization.NOT_DETERMINED -> "notDetermined"
            LocationProviderAuthorization.FOREGROUND_ONLY -> "foregroundOnly"
            LocationProviderAuthorization.AUTHORIZED -> "authorized"
            LocationProviderAuthorization.DENIED -> "denied"
            LocationProviderAuthorization.RESTRICTED -> "restricted"
            LocationProviderAuthorization.UNAVAILABLE -> "unavailable"
        }
        return JSONObject()
            .put("status", status)
            .put(
                "permissionAction",
                when (status) {
                    "notDetermined" -> "requestPermission"
                    "denied", "restricted" -> "openSettings"
                    else -> JSONObject.NULL
                },
            )
            .put("backgroundCapture", value.backgroundCapture.name.lowercase())
            .put("reason", value.reason ?: JSONObject.NULL)
    }

    private fun liveSample(value: TelemetrySample) = JSONObject()
        .put("sequence", value.sequence)
        .put("observedAt", value.observedAt.toString())
        .put("slowObservedAt", value.slowObservedAt?.toString() ?: JSONObject.NULL)
        .put("elapsedMs", value.elapsedMs)
        .put("rideState", value.rideState.name.lowercase())
        .put("packageState", value.packageState?.wireValue ?: JSONObject.NULL)
        .put("packageMode", value.packageMode?.wireValue ?: JSONObject.NULL)
        .put("footpadState", value.footpadState?.wireValue ?: JSONObject.NULL)
        .put("stopCondition", value.stopCondition?.wireValue ?: JSONObject.NULL)
        .put("setpointAdjustment", value.setpointAdjustment?.wireValue ?: JSONObject.NULL)
        .put("wheelslip", value.wheelslip ?: JSONObject.NULL)
        .put("darkride", value.darkride ?: JSONObject.NULL)
        .put("charging", value.charging ?: JSONObject.NULL)
        .put("alertReason", value.alertReason?.wireValue ?: JSONObject.NULL)
        .put("firmwareFault", value.firmwareFault?.wireValue ?: JSONObject.NULL)
        .put("packVoltageV", value.packVoltageV ?: JSONObject.NULL)
        .put("dutyCycle", value.dutyCycle ?: JSONObject.NULL)
        .put("controllerTempC", value.controllerTempC ?: JSONObject.NULL)
        .put("motorTempC", value.motorTempC ?: JSONObject.NULL)
        .put("motorCurrentA", value.motorCurrentA ?: JSONObject.NULL)
        .put("batteryCurrentA", value.batteryCurrentA ?: JSONObject.NULL)
        .put("erpm", value.erpm ?: JSONObject.NULL)
        .put("speedKph", value.speedKph ?: JSONObject.NULL)
        .put("pitchDeg", value.pitchDeg ?: JSONObject.NULL)
        .put("balancePitchDeg", value.balancePitchDeg ?: JSONObject.NULL)
        .put("rollDeg", value.rollDeg ?: JSONObject.NULL)
        .put("directionalMotorCurrentA", value.directionalMotorCurrentA ?: JSONObject.NULL)
        .put("filteredMotorCurrentA", value.filteredMotorCurrentA ?: JSONObject.NULL)
        .put("requestedBalanceCurrentA", value.requestedBalanceCurrentA ?: JSONObject.NULL)
        .put("boosterCurrentA", value.boosterCurrentA ?: JSONObject.NULL)
        .put("fieldWeakeningCurrentA", value.fieldWeakeningCurrentA ?: JSONObject.NULL)
        .put("footpadAdc1V", value.footpadAdc1V ?: JSONObject.NULL)
        .put("footpadAdc2V", value.footpadAdc2V ?: JSONObject.NULL)
        .put("remoteInput", value.remoteInput ?: JSONObject.NULL)
        .put("setpointDeg", value.setpointDeg ?: JSONObject.NULL)
        .put("atrSetpointDeg", value.atrSetpointDeg ?: JSONObject.NULL)
        .put("brakeTiltSetpointDeg", value.brakeTiltSetpointDeg ?: JSONObject.NULL)
        .put("torqueTiltSetpointDeg", value.torqueTiltSetpointDeg ?: JSONObject.NULL)
        .put("turnTiltSetpointDeg", value.turnTiltSetpointDeg ?: JSONObject.NULL)
        .put("remoteSetpointDeg", value.remoteSetpointDeg ?: JSONObject.NULL)
        .put(
            "atrAccelerationDifferenceErpmPerSecond",
            value.atrAccelerationDifferenceErpmPerSecond ?: JSONObject.NULL,
        )
        .put("atrSpeedBoost", value.atrSpeedBoost ?: JSONObject.NULL)
        .put("distanceM", value.distanceM ?: JSONObject.NULL)
        .put("odometerM", value.odometerM ?: JSONObject.NULL)
        .put("ampHoursDrawnAh", value.ampHoursDrawnAh ?: JSONObject.NULL)
        .put("ampHoursChargedAh", value.ampHoursChargedAh ?: JSONObject.NULL)
        .put("wattHoursDrawnWh", value.wattHoursDrawnWh ?: JSONObject.NULL)
        .put("wattHoursChargedWh", value.wattHoursChargedWh ?: JSONObject.NULL)
        .put("batteryLevel", value.batteryLevel ?: JSONObject.NULL)
        .put("chargingCurrentA", value.chargingCurrentA ?: JSONObject.NULL)
        .put("chargingVoltageV", value.chargingVoltageV ?: JSONObject.NULL)
        .put("extensions", JSONObject(value.extension))

    private fun liveLocation(value: LiveLocation) = JSONObject()
        .put("latitude", value.latitude)
        .put("longitude", value.longitude)
        .put("accuracyM", value.accuracyM ?: JSONObject.NULL)
        .put("observedAt", value.observedAt.toString())

    private fun sessionEvent(value: SessionEventRecord): JSONObject =
        JSONObject()
            .put("elapsedMs", value.elapsedMs)
            .put("kind", value.kind)
            .put("value", JSONObject(value.detailsJson).getString("value"))

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
            this is LocationProviderState ||
            this is TelemetrySample ||
            this is fail.acheron.floatview.backend.BmsState ||
            this is fail.acheron.floatview.backend.BmsSnapshot ||
            this is RefloatControls ||
            this === LocationUpdated ||
            this === SessionCatalogChanged ||
            this is VescProtocolMessage.Firmware ||
            this is VescProtocolMessage.CurrentLimits ||
            this is VescProtocolMessage.RefloatInfo ||
            this is VescProtocolMessage.RefloatLighting

    private object SessionCatalogChanged

    private fun encodeSessionCatalogCursor(value: JSONObject): String =
        Base64.encodeToString(value.toString().toByteArray(Charsets.UTF_8), Base64.NO_WRAP)

    private fun decodeSessionCatalogCursor(value: String): JSONObject? {
        if (value.toByteArray(Charsets.UTF_8).size > 1_024) return null
        return runCatching {
            JSONObject(String(Base64.decode(value, Base64.DEFAULT), Charsets.UTF_8)).takeIf {
                it.optInt("version", 0) == 1 &&
                    it.optString("query") == "startedAt.desc.sessionId.desc" &&
                    it.optString("innerCursor").isNotEmpty()
            }
        }.getOrNull()
    }

    private companion object {
        const val MAX_RECENT_ITEMS = 50
        val NUMERIC_FIELDS = listOf(
            "packVoltageV", "dutyCycle", "controllerTempC", "motorTempC",
            "motorCurrentA", "batteryCurrentA", "erpm", "speedKph", "pitchDeg",
            "balancePitchDeg", "rollDeg", "directionalMotorCurrentA",
            "filteredMotorCurrentA", "requestedBalanceCurrentA", "boosterCurrentA",
            "fieldWeakeningCurrentA", "footpadAdc1V", "footpadAdc2V", "remoteInput",
            "setpointDeg", "atrSetpointDeg", "brakeTiltSetpointDeg",
            "torqueTiltSetpointDeg", "turnTiltSetpointDeg", "remoteSetpointDeg",
            "atrAccelerationDifferenceErpmPerSecond", "atrSpeedBoost", "distanceM",
            "odometerM", "ampHoursDrawnAh", "ampHoursChargedAh",
            "wattHoursDrawnWh", "wattHoursChargedWh", "batteryLevel",
            "chargingCurrentA", "chargingVoltageV",
        )
    }
}
