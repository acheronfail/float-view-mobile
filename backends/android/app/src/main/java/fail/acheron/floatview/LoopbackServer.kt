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
import fail.acheron.floatview.backend.SessionEventRecord
import fail.acheron.floatview.backend.SessionElapsedRange
import fail.acheron.floatview.backend.SessionGapPolicyRecord
import fail.acheron.floatview.backend.SessionHistoryCursor
import fail.acheron.floatview.backend.SessionHistoryReducer
import fail.acheron.floatview.backend.SessionPruneMetadataRecord
import fail.acheron.floatview.backend.SessionMaintenanceJobRecord
import fail.acheron.floatview.backend.SessionListQuery
import fail.acheron.floatview.backend.SessionRecord
import fail.acheron.floatview.backend.SessionSeriesPoint
import fail.acheron.floatview.backend.SessionPublishedSnapshotRecord
import fail.acheron.floatview.backend.SessionStoreException
import fail.acheron.floatview.backend.SessionSummaryRecord
import fail.acheron.floatview.backend.SettingsRecord
import fail.acheron.floatview.backend.TelemetrySample
import fail.acheron.floatview.backend.VescProtocolMessage
import fail.acheron.floatview.protocol.refloat.RefloatCompatibility
import fail.acheron.floatview.protocol.vesc.ControllerRoute
import fail.acheron.floatview.protocol.vesc.VescCompatibility
import io.ktor.http.ContentType
import io.ktor.http.HttpStatusCode
import io.ktor.server.application.install
import io.ktor.server.cio.CIO
import io.ktor.server.engine.EmbeddedServer
import io.ktor.server.engine.embeddedServer
import io.ktor.server.request.header
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
import java.util.zip.GZIPOutputStream
import java.util.concurrent.atomic.AtomicInteger
import java.util.UUID
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.selects.select
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject

internal const val ANDROID_BACKEND_PORT = 11012
internal const val LOOPBACK_MAX_MESSAGE_BYTES = 65_536

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
    private val sessionCatalogRevision = AtomicInteger()
    private var server: EmbeddedServer<*, *>? = null
    private var scanStopJob: Job? = null
    private var boundPort = 0
    val port: Int get() = boundPort
    val origin: String get() = "http://127.0.0.1:$port"

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
                            .put("sessionToken", token)
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
                    val body = call.receiveText()
                    if (body.toByteArray(Charsets.UTF_8).size > 16_384) {
                        call.respondText("", status = HttpStatusCode.PayloadTooLarge)
                        return@post
                    }
                    try {
                        val requestPayload = JSONObject(body)
                        val sessionId = requestPayload.optString("sessionId")
                        if (sessionId.isEmpty()) {
                            call.respondText("", status = HttpStatusCode.BadRequest)
                            return@post
                        }
                        val payload = withContext(Dispatchers.IO) {
                            try {
                                sessionData(requestPayload).toString().toByteArray(Charsets.UTF_8)
                            } finally {
                                // Ktor cancellation runs this finally block when
                                // the client disconnects, closing any dedicated
                                // SQLite reader for this request.
                                applicationService.cancelHistoricalReads(sessionId)
                            }
                        }
                        if (payload.size > 4 * 1024 * 1024) {
                            call.respondText("", status = HttpStatusCode.PayloadTooLarge)
                            return@post
                        }
                        if (call.request.header("Accept-Encoding")?.contains("gzip", ignoreCase = true) == true) {
                            val compressed = ByteArrayOutputStream(payload.size)
                            withContext(Dispatchers.IO) {
                                GZIPOutputStream(compressed).use { it.write(payload) }
                            }
                            call.response.headers.append("Content-Encoding", "gzip")
                            call.respondBytes(compressed.toByteArray(), ContentType.Application.Json)
                        } else {
                            call.respondBytes(payload, ContentType.Application.Json)
                        }
                    } catch (_: Exception) {
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
                "session.list" -> {
                    val limit = params.number("limit")?.toInt()
                        ?: return failure(id.toInt(), "invalid_request", "Session page size is required")
                    if (limit !in 1..100) return failure(id.toInt(), "invalid_request", "Session page size must be 1 through 100")
                    val rawCursor = params.optString("cursor").takeIf { params.has("cursor") && !params.isNull("cursor") }
                    val context = rawCursor?.let(::decodeSessionCatalogCursor)
                    if (rawCursor != null && context == null) {
                        return failure(id.toInt(), "invalid_request", "Session cursor is invalid")
                    }
                    if (context != null && context.getLong("catalogRevision") != sessionCatalogRevision.get().toLong()) {
                        return failure(id.toInt(), "conflict", "Session cursor is stale")
                    }
                    val page = applicationService.sessions(
                        SessionListQuery(
                            cursor = context?.getString("innerCursor"),
                            limit = limit,
                            sortField = "startedAt",
                            sortDescending = true,
                        ),
                    )
                    val nextCursor = page.nextCursor?.let { inner ->
                        encodeSessionCatalogCursor(
                            JSONObject()
                                .put("version", 1)
                                .put("query", "startedAt.desc.sessionId.desc")
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
                    success(maintenanceJobJson(job))
                }
                "session.prune.cancel" -> {
                    val job = applicationService.cancelMaintenanceJob(params.optString("operationId"))
                    success(job?.let(::maintenanceJobJson) ?: operation(params.optString("operationId"), "prune", "cancelled", null, null))
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
            .put("sessionOperation", runCatching { applicationService.maintenanceJob() }
                .getOrNull()?.let(::maintenanceJobJson) ?: JSONObject.NULL)
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
        .put("prune", JSONObject()
            .put("targetResolutionSeconds", prune.targetResolutionSeconds ?: JSONObject.NULL)
            .put("recordedSampleCount", prune.recordedSampleCount)
            .put("storedSampleCount", prune.storedSampleCount)
            .put("policy", prune.policy ?: JSONObject.NULL))
        .put("snapshot", JSONObject()
            .put("historyRevision", snapshot.historyRevision)
            .put("committedSampleSequence", snapshot.committedSampleSequence)
            .put("committedEventSequence", snapshot.committedEventSequence)
            .put("committedElapsedMs", snapshot.committedElapsedMs))

    private fun sessionData(params: JSONObject): JSONObject {
        val kind = params.getString("kind")
        val sessionId = params.getString("sessionId")
        val range = params.getJSONObject("range")
        val start = range.getLong("startElapsedMs")
        val end = range.getLong("endElapsedMs")
        val snapshot = params.getJSONObject("snapshot")
        val published = applicationService.sessionSnapshot(sessionId)
        require(snapshot.getLong("historyRevision") == published.historyRevision)
        require(snapshot.getLong("committedSampleSequence") == published.committedSampleSequence)
        require(snapshot.getLong("committedEventSequence") == published.committedEventSequence)
        require(snapshot.getLong("committedElapsedMs") == published.committedElapsedMs)
        val echoedSnapshot = JSONObject()
            .put("historyRevision", snapshot.getLong("historyRevision"))
            .put("committedSampleSequence", snapshot.getLong("committedSampleSequence"))
            .put("committedEventSequence", snapshot.getLong("committedEventSequence"))
            .put("committedElapsedMs", snapshot.getLong("committedElapsedMs"))
        val fields = params.optJSONArray("fields") ?: JSONArray()
        require(fields.length() in 1..16 || kind == "events")
        val fieldNames = (0 until fields.length()).map { fields.getString(it) }
        val extensionFields = applicationService.sessionExtensionFields(sessionId).toSet()
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
                val source = applicationService.querySessionSeriesSource(
                    sessionId, start, end, fieldNames, 200_000,
                    snapshot.getLong("committedSampleSequence"),
                )
                val historyConfiguration = applicationService.sessionHistoryConfiguration(sessionId)
                val reduced = SessionHistoryReducer.reduceSeries(
                    source,
                    fieldNames,
                    SessionElapsedRange(start, end),
                    buckets,
                    SessionGapPolicyRecord(threshold),
                    expectedPersistenceMs = historyConfiguration.expectedIdlePersistenceMs,
                    expectedRidingPersistenceMs = historyConfiguration.expectedRidingPersistenceMs,
                    expectedIdlePersistenceMs = historyConfiguration.expectedIdlePersistenceMs,
                    retainedResolutionSeconds = historyConfiguration.retainedResolutionSeconds,
                )
                val columns = JSONObject()
                fieldNames.forEach { field ->
                    val column = reduced.getValue(field)
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
            val mode = params.optString("mode", "window")
            val points: List<SessionSeriesPoint>
            val nextCursor: String?
            if (mode == "nearest") {
                val target = if (!params.has("targetElapsedMs")) throw SessionStoreException.InvalidRange
                else params.getLong("targetElapsedMs")
                if (target !in start..end) throw SessionStoreException.InvalidRange
                points = applicationService.queryNearestSessionSample(
                    sessionId, target, fieldNames, snapshot.getLong("committedSampleSequence"),
                )?.let { listOf(it) } ?: emptyList()
                nextCursor = null
            } else {
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
                points = page.first
                nextCursor = page.second?.let(SessionHistoryReducer::encodeCursor)
            }
            val values = JSONObject()
            fieldNames.forEach { field -> values.put(field, JSONArray(points.map { it.values[field] ?: JSONObject.NULL })) }
            return base.put("mode", mode)
                .put("elapsedMs", JSONArray(points.map { it.elapsedMs }))
                .put("sequence", JSONArray(points.map { it.sequence }))
                .put("values", values)
                .put("nextCursor", nextCursor ?: JSONObject.NULL)
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
            this is TelemetrySample ||
            this === SessionCatalogChanged ||
            this is VescProtocolMessage.Firmware ||
            this is VescProtocolMessage.CurrentLimits ||
            this is VescProtocolMessage.RefloatInfo

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
