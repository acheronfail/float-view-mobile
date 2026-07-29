package fail.acheron.floatview.storage

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import fail.acheron.floatview.backend.DeviceId
import fail.acheron.floatview.backend.BoardProfileRecord
import fail.acheron.floatview.backend.ControllerRoutingHint
import fail.acheron.floatview.backend.JobId
import fail.acheron.floatview.backend.LogStorePort
import fail.acheron.floatview.backend.ProductStoreException
import fail.acheron.floatview.backend.ProductStorePort
import fail.acheron.floatview.backend.SavedDeviceRecord
import fail.acheron.floatview.backend.SessionId
import fail.acheron.floatview.backend.SessionEventRecord
import fail.acheron.floatview.backend.SessionExportRecord
import fail.acheron.floatview.backend.SessionGroupRecord
import fail.acheron.floatview.backend.SessionListQuery
import fail.acheron.floatview.backend.SessionPage
import fail.acheron.floatview.backend.SessionRecord
import fail.acheron.floatview.backend.SessionSeriesPoint
import fail.acheron.floatview.backend.SessionStoreException
import fail.acheron.floatview.backend.SessionStorePort
import fail.acheron.floatview.backend.SessionSummaryRecord
import fail.acheron.floatview.backend.SettingsRecord
import fail.acheron.floatview.backend.ThemePreference
import fail.acheron.floatview.backend.UnitPreference
import fail.acheron.floatview.backend.TelemetrySample
import android.util.Base64
import org.json.JSONObject
import java.io.File
import java.io.BufferedWriter
import java.io.FileOutputStream
import java.io.OutputStreamWriter
import java.time.Instant
import java.util.UUID

/**
 * Production durable store. The backend owns this object; no Activity or WebView
 * reference is retained. Catalog and session commits are bounded independently.
 */
class SQLiteLogStore(
    context: Context,
    private val availableBytes: () -> Long = { context.filesDir.usableSpace },
    rootDirectory: File = File(context.filesDir, "float-view/logs"),
) : LogStorePort, ProductStorePort, SessionStorePort {
    private val appContext = context.applicationContext
    private val root = prepareStorageRoot(rootDirectory)
    private val catalog = CatalogOpenHelper(appContext, File(root, "catalog.sqlite").path)
    private var current: SessionRecord? = null
    private var sessionDb: SQLiteDatabase? = null

    init {
        resumePendingDeletes(catalog.writableDatabase)
        cleanupStagedExports()
    }

    @Synchronized
    override fun create(deviceId: DeviceId, advertisedName: String?, at: Instant): SessionRecord {
        check(StoragePolicy.level(availableBytes()) != StorageLevel.STOP) {
            "Storage below durable logging stop threshold"
        }
        current?.let { return it }
        val sessionId = SessionId(UUID.randomUUID().toString())
        val jobId = JobId(UUID.randomUUID().toString())
        val fileName = "session-${sessionId.value}.sqlite"
        val sessionFile = File(root, fileName)
        // Force catalog creation before creating the per-session file so a
        // catalog configuration failure cannot leave an orphan.
        val catalogDb = catalog.writableDatabase
        val db = SQLiteDatabase.openOrCreateDatabase(sessionFile, null)
        try {
            check(db.enableWriteAheadLogging()) {
                "Unable to enable WAL for durable session logging"
            }
            executePragma(db, "PRAGMA synchronous = FULL")
            executeScript(db, asset("session.sql"), skipPragmas = true)
        } catch (error: Exception) {
            db.close()
            SQLiteDatabase.deleteDatabase(sessionFile)
            throw error
        }
        val started = at.toEpochMilli()
        catalogDb.beginTransaction()
        try {
            val savedDevice = ensureSavedDevice(catalogDb, deviceId, advertisedName, at)
            val profile = boardProfile(catalogDb, savedDevice.savedDeviceId, at)
            val settings = settings(catalogDb)
            val deviceSnapshot = JSONObject()
                .put("savedDeviceId", savedDevice.savedDeviceId)
                .put("deviceHistoryId", savedDevice.deviceHistoryId)
                .put("displayName", savedDevice.displayName)
                .toString()
            catalogDb.execSQL(
                """
                INSERT INTO session_catalog(
                  session_id, job_id, device_history_id, device_snapshot_json,
                  profile_snapshot_json, profile_revision, file_name, started_at_ms,
                  state, integrity, requested_idle_ms, effective_idle_ms,
                  requested_riding_ms, effective_riding_ms
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'active', 'complete', ?, ?, ?, ?)
                """.trimIndent(),
                arrayOf(
                    sessionId.value,
                    jobId.value,
                    savedDevice.deviceHistoryId,
                    deviceSnapshot,
                    profile.valuesJson,
                    profile.revision,
                    fileName,
                    started,
                    settings.idlePersistIntervalMs,
                    settings.idlePersistIntervalMs,
                    settings.ridingPersistIntervalMs,
                    settings.ridingPersistIntervalMs,
                ),
            )
            catalogDb.setTransactionSuccessful()
        } catch (error: Exception) {
            db.close()
            SQLiteDatabase.deleteDatabase(sessionFile)
            throw error
        } finally {
            catalogDb.endTransaction()
        }
        sessionDb = db
        return SessionRecord(sessionId, jobId, deviceId, at, 0, true).also { current = it }
    }

    fun create(deviceId: DeviceId, at: Instant): SessionRecord = create(deviceId, null, at)

    @Synchronized
    override fun ensureSavedDevice(
        deviceId: DeviceId,
        advertisedName: String?,
        at: Instant,
    ): SavedDeviceRecord = ensureSavedDevice(
        catalog.writableDatabase,
        deviceId,
        advertisedName,
        at,
    )

    @Synchronized
    override fun append(jobId: JobId, samples: List<TelemetrySample>) {
        val record = requireNotNull(current) { "No active session" }
        require(record.jobId == jobId) { "Wrong logging job" }
        if (samples.isEmpty()) return
        check(StoragePolicy.level(availableBytes()) != StorageLevel.STOP) {
            "Storage below durable logging stop threshold"
        }
        val db = requireNotNull(sessionDb)
        var appendedEvents = 0L
        db.beginTransaction()
        try {
            var previousTransitions = latestTransitionValues(db).toMutableMap()
            var eventSequence = android.database.DatabaseUtils.longForQuery(
                db,
                "SELECT COALESCE(MAX(sequence), 0) + 1 FROM session_event",
                null,
            )
            for (sample in samples) {
                val detailJson = telemetryDetailJson(sample)
                val extensionJson = telemetryExtensionJson(sample.extension)
                db.execSQL(
                    """
                    INSERT OR IGNORE INTO sample(
                      sequence, observed_at_ms, slow_observed_at_ms, monotonic_elapsed_ms,
                      ride_state, package_state, package_mode, stop_condition,
                      setpoint_adjustment, wheelslip, darkride, charging, alert_reason,
                      firmware_fault, raw_pack_voltage_v, duty_cycle, controller_temp_c,
                      motor_temp_c, footpad_state, detail_json, extension_json,
                      motor_current_a, battery_current_a, erpm, speed_kph
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """.trimIndent(),
                    arrayOf(
                        sample.sequence,
                        sample.observedAt.toEpochMilli(),
                        sample.slowObservedAt?.toEpochMilli(),
                        sample.elapsedMs,
                        sample.rideState.name.lowercase(),
                        sample.packageState?.wireValue,
                        sample.packageMode?.wireValue,
                        sample.stopCondition?.wireValue,
                        sample.setpointAdjustment?.wireValue,
                        sample.wheelslip?.let { if (it) 1 else 0 },
                        sample.darkride?.let { if (it) 1 else 0 },
                        sample.charging?.let { if (it) 1 else 0 },
                        sample.alertReason?.wireValue,
                        sample.firmwareFault?.wireValue,
                        sample.packVoltageV,
                        sample.dutyCycle,
                        sample.controllerTempC,
                        sample.motorTempC,
                        sample.footpadState?.wireValue,
                        detailJson,
                        extensionJson,
                        sample.motorCurrentA,
                        sample.batteryCurrentA,
                        sample.erpm,
                        sample.speedKph,
                    ),
                )
                val inserted = android.database.DatabaseUtils.longForQuery(
                    db,
                    "SELECT changes()",
                    null,
                ) == 1L
                if (!inserted) continue
                transitionValues(sample).forEach { (kind, value) ->
                    if (previousTransitions[kind] == value) return@forEach
                    db.execSQL(
                        """
                        INSERT INTO session_event(
                          sequence, observed_at_ms, monotonic_elapsed_ms, kind, details_json
                        ) VALUES (?, ?, ?, ?, ?)
                        """.trimIndent(),
                        arrayOf(
                            eventSequence,
                            sample.observedAt.toEpochMilli(),
                            sample.elapsedMs,
                            kind,
                            JSONObject().put("value", value).toString(),
                        ),
                    )
                    previousTransitions[kind] = value
                    eventSequence += 1
                    appendedEvents += 1
                }
            }
            val last = samples.maxOf { it.sequence }
            db.execSQL(
                "INSERT OR REPLACE INTO commit_marker(singleton, committed_sequence, committed_at_ms) VALUES (1, ?, ?)",
                arrayOf(last, samples.last().observedAt.toEpochMilli()),
            )
            db.setTransactionSuccessful()
            current = record.copy(committedSequence = maxOf(record.committedSequence, last))
        } finally {
            db.endTransaction()
        }
        updateCatalogSummary(record, samples)
        if (appendedEvents > 0) {
            catalog.writableDatabase.execSQL(
                """
                UPDATE session_catalog SET event_count = event_count + ?
                WHERE job_id = ? AND state = 'active'
                """.trimIndent(),
                arrayOf(appendedEvents, record.jobId.value),
            )
        }
    }

    @Synchronized
    override fun close(jobId: JobId, at: Instant, reason: String): SessionRecord {
        val record = requireNotNull(current) { "No active session" }
        require(record.jobId == jobId) { "Wrong logging job" }
        sessionDb?.let { executePragma(it, "PRAGMA wal_checkpoint(FULL)") }
        sessionDb?.close()
        sessionDb = null
        val catalogDb = catalog.writableDatabase
        catalogDb.beginTransaction()
        try {
            catalogDb.execSQL(
                """
                UPDATE session_catalog
                SET ended_at_ms = ?, state = 'completed', committed_sequence = ?, close_reason = ?
                WHERE job_id = ? AND state = 'active'
                """.trimIndent(),
                arrayOf(at.toEpochMilli(), record.committedSequence, reason, jobId.value),
            )
            linkClosedSession(catalogDb, record.sessionId.value)
            catalogDb.setTransactionSuccessful()
        } finally {
            catalogDb.endTransaction()
        }
        return record.copy(active = false).also { current = null }
    }

    @Synchronized
    override fun active(): SessionRecord? = current

    @Synchronized
    override fun sessions(query: SessionListQuery): SessionPage {
        if (query.limit !in 1..100) throw SessionStoreException.InvalidRange
        val sortColumn = SESSION_SORT_COLUMNS[query.sortField]
            ?: throw SessionStoreException.InvalidRange
        val cursor = query.cursor?.let(::decodeCursor)
        if (cursor != null && cursor.first != query.sortField) {
            throw SessionStoreException.InvalidCursor
        }
        val conditions = mutableListOf("pending_delete = 0")
        val values = mutableListOf<String>()
        if (cursor != null) {
            val comparison = if (query.sortDescending) "<" else ">"
            conditions +=
                "($sortColumn $comparison ? OR ($sortColumn = ? AND session_id $comparison ?))"
            values += cursor.second.toString()
            values += cursor.second.toString()
            values += cursor.third
        }
        query.deviceHistoryId?.let {
            conditions += "device_history_id = ?"; values += it
        }
        query.startedAfter?.let {
            conditions += "started_at_ms >= ?"; values += it.toEpochMilli().toString()
        }
        query.startedBefore?.let {
            conditions += "started_at_ms <= ?"; values += it.toEpochMilli().toString()
        }
        query.minimumTotalElapsedMs?.let {
            conditions += "total_elapsed_ms >= ?"; values += it.toString()
        }
        query.maximumTotalElapsedMs?.let {
            conditions += "total_elapsed_ms <= ?"; values += it.toString()
        }
        query.minimumRidingElapsedMs?.let {
            conditions += "riding_elapsed_ms >= ?"; values += it.toString()
        }
        query.maximumRidingElapsedMs?.let {
            conditions += "riding_elapsed_ms <= ?"; values += it.toString()
        }
        query.minimumDistanceM?.let {
            conditions += "distance_m >= ?"; values += it.toString()
        }
        query.maximumDistanceM?.let {
            conditions += "distance_m <= ?"; values += it.toString()
        }
        query.state?.let { conditions += "state = ?"; values += it }
        query.integrity?.let { conditions += "integrity = ?"; values += it }
        val where = if (conditions.isEmpty()) "" else "WHERE ${conditions.joinToString(" AND ")}"
        values += (query.limit + 1).toString()
        val rows = catalog.writableDatabase.rawQuery(
            """
            SELECT session_id, device_history_id, device_snapshot_json, started_at_ms,
                   ended_at_ms, state, integrity, sample_count, event_count,
                   riding_elapsed_ms, total_elapsed_ms, distance_m, max_speed_mps,
                   max_duty_cycle
            FROM session_catalog $where
            ORDER BY $sortColumn ${if (query.sortDescending) "DESC" else "ASC"},
                     session_id ${if (query.sortDescending) "DESC" else "ASC"} LIMIT ?
            """.trimIndent(),
            values.toTypedArray(),
        ).use { cursorValue ->
            buildList {
                while (cursorValue.moveToNext()) add(cursorValue.sessionSummary())
            }
        }.toMutableList()
        val hasMore = rows.size > query.limit
        if (hasMore) rows.removeAt(rows.lastIndex)
        return SessionPage(
            rows,
            if (hasMore) rows.lastOrNull()?.let { encodeCursor(it, query.sortField) } else null,
        )
    }

    @Synchronized
    override fun session(sessionId: String): SessionSummaryRecord =
        catalog.writableDatabase.rawQuery(
            """
            SELECT session_id, device_history_id, device_snapshot_json, started_at_ms,
                   ended_at_ms, state, integrity, sample_count, event_count,
                   riding_elapsed_ms, total_elapsed_ms, distance_m, max_speed_mps,
                   max_duty_cycle
            FROM session_catalog WHERE session_id = ? AND pending_delete = 0
            """.trimIndent(),
            arrayOf(sessionId),
        ).use { cursor ->
            if (!cursor.moveToFirst()) throw SessionStoreException.SessionNotFound
            cursor.sessionSummary()
        }

    @Synchronized
    override fun querySeries(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        fields: List<String>,
        maxPoints: Int,
    ): List<SessionSeriesPoint> {
        if (startElapsedMs > endElapsedMs || maxPoints !in 1..2_000) {
            throw SessionStoreException.InvalidRange
        }
        return readSeries(sessionId, startElapsedMs, endElapsedMs, fields, maxPoints, false)
    }

    @Synchronized
    override fun diagnosticWindow(
        sessionId: String,
        centerElapsedMs: Long,
        beforeMs: Long,
        afterMs: Long,
        fields: List<String>,
    ): List<SessionSeriesPoint> {
        if (centerElapsedMs < 0 || beforeMs !in 0..300_000 || afterMs !in 0..300_000) {
            throw SessionStoreException.InvalidRange
        }
        return readSeries(
            sessionId,
            maxOf(0, centerElapsedMs - beforeMs),
            centerElapsedMs + afterMs,
            fields,
            2_000,
            true,
        )
    }

    @Synchronized
    override fun sessionEvents(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
    ): List<SessionEventRecord> {
        if (startElapsedMs > endElapsedMs) throw SessionStoreException.InvalidRange
        return openSession(sessionId).use { db ->
            db.rawQuery(
                """
                SELECT sequence, monotonic_elapsed_ms, kind, details_json FROM (
                  SELECT sequence, monotonic_elapsed_ms, kind, details_json
                  FROM session_event WHERE monotonic_elapsed_ms BETWEEN ? AND ?
                    AND kind IN (
                      'packageState', 'packageMode', 'footpadState', 'stopCondition',
                      'setpointAdjustment', 'wheelslip', 'darkride', 'charging',
                      'alertReason', 'firmwareFault'
                    )
                  ORDER BY sequence DESC LIMIT 2001
                ) ORDER BY monotonic_elapsed_ms, sequence
                """.trimIndent(),
                arrayOf(startElapsedMs.toString(), endElapsedMs.toString()),
            ).use { cursor ->
                buildList {
                    while (cursor.moveToNext()) {
                        add(
                            SessionEventRecord(
                                cursor.getLong(0).toString(),
                                cursor.getLong(1),
                                cursor.getString(2),
                                cursor.getString(3),
                            ),
                        )
                    }
                }
            }
        }
    }

    @Synchronized
    override fun sessionExtensionFields(sessionId: String): List<String> =
        openSession(sessionId).use { db ->
            db.rawQuery(
                """
                SELECT DISTINCT json_each.key FROM sample, json_each(sample.extension_json)
                ORDER BY json_each.key LIMIT 33
                """.trimIndent(),
                null,
            ).use { cursor ->
                buildList {
                    while (cursor.moveToNext()) add(cursor.getString(0))
                }.also {
                    check(it.size <= 32) { "Stored telemetry extensions exceed their bound" }
                }
            }
        }

    @Synchronized
    override fun savedDevices(): List<SavedDeviceRecord> {
        val db = catalog.writableDatabase
        return db.rawQuery(
            """
            SELECT d.saved_device_id, d.device_history_id, d.revision,
                   d.custom_display_name, d.advertised_name, d.opaque_platform_id,
                   d.last_seen_at_ms, d.last_connected_at_ms, COALESCE(p.json, '{}')
            FROM saved_device d
            LEFT JOIN board_profile p ON p.saved_device_id = d.saved_device_id
            ORDER BY COALESCE(d.last_connected_at_ms, d.created_at_ms) DESC
            """.trimIndent(),
            null,
        ).use { cursor ->
            buildList {
                while (cursor.moveToNext()) {
                    val valuesJson = cursor.getString(8)
                    val customName = cursor.optionalString(3)
                    val advertisedName = cursor.optionalString(4)
                    val identifier = cursor.getString(5)
                    add(
                        SavedDeviceRecord(
                            savedDeviceId = cursor.getString(0),
                            deviceHistoryId = cursor.getString(1),
                            revision = cursor.getInt(2),
                            displayName = resolvedDisplayName(customName, advertisedName, identifier),
                            customDisplayName = customName,
                            advertisedName = advertisedName,
                            bluetoothIdentifier = identifier,
                            lastSeenAt = cursor.optionalInstant(6),
                            lastConnectedAt = cursor.optionalInstant(7),
                            profileComplete = missingFields(valuesJson).isEmpty(),
                        ),
                    )
                }
            }
        }
    }

    @Synchronized
    override fun updateSavedDevice(
        savedDeviceId: String,
        expectedRevision: Int,
        customDisplayName: String?,
        at: Instant,
    ): SavedDeviceRecord {
        val normalized = customDisplayName?.trim()?.takeIf(String::isNotEmpty)
        if ((normalized?.toByteArray()?.size ?: 0) > 128) {
            throw ProductStoreException.InvalidSavedDevicePatch
        }
        val db = catalog.writableDatabase
        db.beginTransaction()
        return try {
            db.execSQL(
                """
                UPDATE saved_device
                SET revision = revision + 1, custom_display_name = ?
                WHERE saved_device_id = ? AND revision = ?
                """.trimIndent(),
                arrayOf(normalized, savedDeviceId, expectedRevision),
            )
            val changed = android.database.DatabaseUtils.longForQuery(db, "SELECT changes()", null)
            if (changed != 1L) {
                val current = savedDevices().firstOrNull { it.savedDeviceId == savedDeviceId }
                    ?: throw ProductStoreException.SavedDeviceNotFound
                throw ProductStoreException.SavedDeviceRevisionConflict(current.revision)
            }
            savedDevices().first { it.savedDeviceId == savedDeviceId }
                .also { db.setTransactionSuccessful() }
        } finally {
            db.endTransaction()
        }
    }

    @Synchronized
    override fun boardProfile(savedDeviceId: String): BoardProfileRecord =
        boardProfile(catalog.writableDatabase, savedDeviceId, Instant.now())

    @Synchronized
    override fun updateBoardProfile(
        savedDeviceId: String,
        expectedRevision: Int,
        patchJson: String,
        at: Instant,
    ): BoardProfileRecord {
        val patch = try {
            JSONObject(patchJson)
        } catch (_: Exception) {
            throw ProductStoreException.InvalidProfilePatch
        }
        if (patchJson.toByteArray().size > MAX_PROFILE_BYTES) {
            throw ProductStoreException.InvalidProfilePatch
        }
        val db = catalog.writableDatabase
        db.beginTransaction()
        val updated = try {
            val existing = boardProfile(db, savedDeviceId, at)
            if (existing.revision != expectedRevision) {
                throw ProductStoreException.ProfileRevisionConflict(existing.revision)
            }
            val values = try {
                JSONObject(existing.valuesJson)
            } catch (_: Exception) {
                throw ProductStoreException.InvalidProfilePatch
            }
            val oldPreferredCanId = preferredCanId(values)
            patch.keys().forEach { key -> values.put(key, patch.get(key)) }
            if (!validInitialProfileValues(values)) {
                throw ProductStoreException.InvalidProfilePatch
            }
            val newPreferredCanId = preferredCanId(values)
            val valuesJson = values.toString()
            if (valuesJson.toByteArray().size > MAX_PROFILE_BYTES) {
                throw ProductStoreException.InvalidProfilePatch
            }
            val revision = existing.revision + 1
            db.execSQL(
                """
                UPDATE board_profile SET revision = ?, json = ?, updated_at_ms = ?
                WHERE saved_device_id = ? AND revision = ?
                """.trimIndent(),
                arrayOf(revision, valuesJson, at.toEpochMilli(), savedDeviceId, expectedRevision),
            )
            val changed = android.database.DatabaseUtils.longForQuery(
                db,
                "SELECT changes()",
                null,
            )
            if (changed != 1L) {
                throw ProductStoreException.ProfileRevisionConflict(existing.revision)
            }
            if (
                patch.has("preferredCanId") &&
                (oldPreferredCanId != newPreferredCanId || newPreferredCanId == null)
            ) {
                clearControllerRoutingHint(db, savedDeviceId)
            }
            BoardProfileRecord(
                savedDeviceId,
                revision,
                valuesJson,
                missingFields(valuesJson),
            ).also { db.setTransactionSuccessful() }
        } finally {
            db.endTransaction()
        }
        recordProfileChange(updated, at)
        return updated
    }

    @Synchronized
    override fun selectController(
        savedDeviceId: String,
        canId: Int,
        controllerUuid: ByteArray?,
        at: Instant,
    ): BoardProfileRecord {
        if (canId !in 0..253 || (controllerUuid != null && controllerUuid.size != 12)) {
            throw ProductStoreException.InvalidProfilePatch
        }
        val db = catalog.writableDatabase
        db.beginTransaction()
        val updated = try {
            val current = boardProfile(db, savedDeviceId, at)
            val values = try {
                JSONObject(current.valuesJson)
            } catch (_: Exception) {
                throw ProductStoreException.InvalidProfilePatch
            }
            values.put("preferredCanId", canId)
            val valuesJson = values.toString()
            val revision = current.revision + 1
            db.execSQL(
                """
                UPDATE board_profile SET revision = ?, json = ?, updated_at_ms = ?
                WHERE saved_device_id = ? AND revision = ?
                """.trimIndent(),
                arrayOf(revision, valuesJson, at.toEpochMilli(), savedDeviceId, current.revision),
            )
            val changed = android.database.DatabaseUtils.longForQuery(db, "SELECT changes()", null)
            if (changed != 1L) {
                throw ProductStoreException.ProfileRevisionConflict(current.revision)
            }
            storeControllerRoutingHint(
                db,
                ControllerRoutingHint(savedDeviceId, canId, controllerUuid),
                at,
            )
            BoardProfileRecord(
                savedDeviceId,
                revision,
                valuesJson,
                missingFields(valuesJson),
            ).also { db.setTransactionSuccessful() }
        } finally {
            db.endTransaction()
        }
        recordProfileChange(updated, at)
        return updated
    }

    @Synchronized
    override fun controllerRoutingHint(savedDeviceId: String): ControllerRoutingHint? =
        catalog.readableDatabase.rawQuery(
            """
            SELECT can_id, controller_uuid
            FROM controller_route_hint WHERE saved_device_id = ? LIMIT 1
            """.trimIndent(),
            arrayOf(savedDeviceId),
        ).use { cursor ->
            if (!cursor.moveToFirst()) return@use null
            ControllerRoutingHint(
                savedDeviceId,
                cursor.getInt(0),
                cursor.optionalString(1)?.hexToBytes(),
            )
        }

    @Synchronized
    override fun storeControllerRoutingHint(hint: ControllerRoutingHint, at: Instant) {
        storeControllerRoutingHint(catalog.writableDatabase, hint, at)
    }

    @Synchronized
    override fun clearControllerRoutingHint(savedDeviceId: String) {
        clearControllerRoutingHint(catalog.writableDatabase, savedDeviceId)
    }

    @Synchronized
    override fun settings(): SettingsRecord = settings(catalog.writableDatabase)

    @Synchronized
    override fun updateSettings(expectedRevision: Int, patchJson: String): SettingsRecord {
        val patch = try {
            JSONObject(patchJson)
        } catch (_: Exception) {
            throw ProductStoreException.InvalidSettingsPatch
        }
        if (patch.length() == 0 || !patch.keys().asSequence().all(SETTING_KEYS::contains)) {
            throw ProductStoreException.InvalidSettingsPatch
        }
        val db = catalog.writableDatabase
        db.beginTransaction()
        return try {
            val current = settings(db)
            if (current.revision != expectedRevision) {
                throw ProductStoreException.SettingsRevisionConflict(current.revision)
            }
            val values = settingsJson(current)
            patch.keys().forEach { values.put(it, patch.get(it)) }
            val updated = validatedSettings(values, current.revision + 1)
            db.execSQL(
                "UPDATE settings SET revision = ?, json = ? WHERE singleton = 1 AND revision = ?",
                arrayOf(updated.revision, settingsJson(updated).put("revision", updated.revision).toString(), expectedRevision),
            )
            val changed = android.database.DatabaseUtils.longForQuery(db, "SELECT changes()", null)
            if (changed != 1L) {
                throw ProductStoreException.SettingsRevisionConflict(current.revision)
            }
            db.setTransactionSuccessful()
            updated
        } finally {
            db.endTransaction()
        }
    }

    @Synchronized
    override fun deleteSession(sessionId: String) {
        if (current?.sessionId?.value == sessionId) throw SessionStoreException.SessionActive
        val db = catalog.writableDatabase
        val sessionRow = db.rawQuery(
            "SELECT file_name, state FROM session_catalog WHERE session_id = ?",
            arrayOf(sessionId),
        ).use { cursor ->
            if (cursor.moveToFirst()) cursor.getString(0) to cursor.getString(1) else null
        } ?: return
        if (sessionRow.second == "active") throw SessionStoreException.SessionActive
        val fileName = sessionRow.first
        db.execSQL(
            "UPDATE session_catalog SET pending_delete = 1 WHERE session_id = ?",
            arrayOf(sessionId),
        )
        val sessionFile = File(root, fileName)
        if (sessionFile.exists() && !SQLiteDatabase.deleteDatabase(sessionFile)) {
            error("Unable to remove durable session files")
        }
        db.beginTransaction()
        try {
            db.execSQL("DELETE FROM session_catalog WHERE session_id = ?", arrayOf(sessionId))
            removeUndersizedGroups(db)
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
    }

    @Synchronized
    override fun sessionGroups(): List<SessionGroupRecord> {
        val db = catalog.writableDatabase
        val ids = db.rawQuery(
            "SELECT session_group_id FROM session_group ORDER BY created_at_ms DESC LIMIT 1000",
            null,
        ).use { cursor ->
            buildList { while (cursor.moveToNext()) add(cursor.getString(0)) }
        }
        return ids.map { sessionGroup(db, it) }
    }

    @Synchronized
    override fun sessionGroup(sessionGroupId: String): SessionGroupRecord =
        sessionGroup(catalog.writableDatabase, sessionGroupId)

    @Synchronized
    override fun unlinkSessionGroup(sessionGroupId: String) {
        catalog.writableDatabase.execSQL(
            "DELETE FROM session_group WHERE session_group_id = ?",
            arrayOf(sessionGroupId),
        )
    }

    @Synchronized
    override fun exportSession(sessionId: String, format: String): SessionExportRecord {
        if (format != "float-view-archive" && format != "csv") {
            throw SessionStoreException.InvalidExportFormat
        }
        if (current?.sessionId?.value == sessionId) throw SessionStoreException.SessionActive
        val db = catalog.writableDatabase
        val sessionRow = db.rawQuery(
            """
            SELECT file_name, state FROM session_catalog
            WHERE session_id = ? AND pending_delete = 0
            """.trimIndent(),
            arrayOf(sessionId),
        ).use { cursor ->
            if (cursor.moveToFirst()) cursor.getString(0) to cursor.getString(1) else null
        } ?: throw SessionStoreException.SessionNotFound
        if (sessionRow.second == "active") throw SessionStoreException.SessionActive
        val fileName = sessionRow.first
        val exports = File(root, "Exports").apply { mkdirs() }
        val exportId = UUID.randomUUID().toString()
        val outputName = "float-view-$exportId.${if (format == "csv") "csv" else "floatview"}"
        val output = File(exports, outputName)
        try {
            openSession(sessionId).use { session ->
                executePragma(session, "PRAGMA wal_checkpoint(FULL)")
                if (format == "csv") {
                    writeCsv(session, output)
                }
            }
            if (format == "float-view-archive") {
                File(root, fileName).inputStream().use { input ->
                    FileOutputStream(output).use { stream -> input.copyTo(stream, 64 * 1024) }
                }
                SQLiteDatabase.openDatabase(
                    output.path,
                    null,
                    SQLiteDatabase.OPEN_READWRITE,
                ).use { archive ->
                    archive.execSQL(
                        """
                        CREATE TABLE IF NOT EXISTS float_view_archive_metadata(
                          key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL
                        )
                        """.trimIndent(),
                    )
                    archive.execSQL(
                        "INSERT OR REPLACE INTO float_view_archive_metadata(key, value) VALUES ('format_version', '1')",
                    )
                    archive.execSQL(
                        "INSERT OR REPLACE INTO float_view_archive_metadata(key, value) VALUES ('session', ?)",
                        arrayOf(sessionArchiveManifest(db, sessionId).toString()),
                    )
                }
            }
            return SessionExportRecord(exportId, outputName, output.path)
        } catch (error: Exception) {
            output.delete()
            throw error
        }
    }

    fun recoverInterrupted(): Int {
        val db = catalog.writableDatabase
        db.execSQL(
            "UPDATE session_catalog SET state = 'interrupted', integrity = 'recovered', close_reason = 'process_interruption' WHERE state = 'active'",
        )
        return android.database.DatabaseUtils.longForQuery(
            db,
            "SELECT changes()",
            null,
        ).toInt()
    }

    private fun settings(db: SQLiteDatabase): SettingsRecord {
        val existing = db.rawQuery(
            "SELECT json FROM settings WHERE singleton = 1",
            null,
        ).use { cursor ->
            if (cursor.moveToFirst()) {
                try {
                    validatedSettings(JSONObject(cursor.getString(0)), JSONObject(cursor.getString(0)).optInt("revision", 0))
                } catch (_: Exception) {
                    null
                }
            } else {
                null
            }
        }
        if (existing != null) return existing
        val value = SettingsRecord()
        db.execSQL(
            "INSERT OR REPLACE INTO settings(singleton, revision, json) VALUES (1, 0, ?)",
            arrayOf(settingsJson(value).put("revision", value.revision).toString()),
        )
        return value
    }

    private fun settingsJson(value: SettingsRecord): JSONObject = JSONObject()
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

    private fun validatedSettings(values: JSONObject, revision: Int): SettingsRecord {
        fun integer(name: String, range: LongRange): Long {
            val raw = values.opt(name)
            if (raw !is Number || raw.toDouble() % 1.0 != 0.0) {
                throw ProductStoreException.InvalidSettingsPatch
            }
            return raw.toLong().coerceIn(range)
        }
        fun boolean(name: String): Boolean =
            (values.opt(name) as? Boolean) ?: throw ProductStoreException.InvalidSettingsPatch
        fun themePreference(name: String): ThemePreference =
            (values.opt(name) as? String)
                ?.let(ThemePreference::fromWire)
                ?: throw ProductStoreException.InvalidSettingsPatch
        fun unitPreference(name: String): UnitPreference =
            (values.opt(name) as? String)
                ?.let(UnitPreference::fromWire)
                ?: throw ProductStoreException.InvalidSettingsPatch
        return SettingsRecord(
            revision = revision,
            idleProbeIntervalMs = integer("idleProbeIntervalMs", 100L..10_000L),
            idlePersistIntervalMs = integer("idlePersistIntervalMs", 100L..60_000L),
            ridingPersistIntervalMs = integer("ridingPersistIntervalMs", 50L..10_000L),
            postRideTailMs = integer("postRideTailMs", 0L..300_000L),
            reconnectDurationMs = integer("reconnectDurationMs", 0L..3_600_000L),
            automaticSessionLinkingEnabled = boolean("automaticSessionLinkingEnabled"),
            sessionLinkWindowMs = integer("sessionLinkWindowMs", 60_000L..86_400_000L),
            themePreference = themePreference("themePreference"),
            unitPreference = unitPreference("unitPreference"),
        )
    }

    private fun sessionGroup(db: SQLiteDatabase, sessionGroupId: String): SessionGroupRecord {
        data class Header(val deviceHistoryId: String, val elapsedMs: Long, val distanceM: Double?)
        val header = db.rawQuery(
            """
            SELECT g.device_history_id, COALESCE(SUM(s.total_elapsed_ms), 0),
                   CASE WHEN COUNT(s.distance_m) = COUNT(*) THEN SUM(s.distance_m) ELSE NULL END
            FROM session_group g
            JOIN session_group_member m ON m.session_group_id = g.session_group_id
            JOIN session_catalog s ON s.session_id = m.session_id AND s.pending_delete = 0
            WHERE g.session_group_id = ?
            GROUP BY g.session_group_id, g.device_history_id
            """.trimIndent(),
            arrayOf(sessionGroupId),
        ).use { cursor ->
            if (!cursor.moveToFirst()) null else Header(
                cursor.getString(0),
                cursor.getLong(1),
                cursor.optionalDouble(2),
            )
        } ?: throw SessionStoreException.SessionNotFound
        val sessionIds = db.rawQuery(
            """
            SELECT m.session_id FROM session_group_member m
            JOIN session_catalog s ON s.session_id = m.session_id AND s.pending_delete = 0
            WHERE m.session_group_id = ? ORDER BY m.ordinal
            """.trimIndent(),
            arrayOf(sessionGroupId),
        ).use { cursor ->
            buildList { while (cursor.moveToNext()) add(cursor.getString(0)) }
        }
        if (sessionIds.size < 2) throw SessionStoreException.SessionNotFound
        return SessionGroupRecord(
            sessionGroupId,
            header.deviceHistoryId,
            sessionIds,
            header.elapsedMs,
            header.distanceM,
        )
    }

    private fun linkClosedSession(db: SQLiteDatabase, sessionId: String) {
        val settings = settings(db)
        if (!settings.automaticSessionLinkingEnabled) return
        val currentSession = db.rawQuery(
            """
            SELECT device_history_id, started_at_ms FROM session_catalog
            WHERE session_id = ? AND pending_delete = 0
            """.trimIndent(),
            arrayOf(sessionId),
        ).use { cursor ->
            if (cursor.moveToFirst()) cursor.getString(0) to cursor.getLong(1) else null
        } ?: return
        val previous = db.rawQuery(
            """
            SELECT session_id, ended_at_ms FROM session_catalog
            WHERE device_history_id = ? AND session_id != ? AND pending_delete = 0
              AND ended_at_ms IS NOT NULL AND ended_at_ms <= ?
            ORDER BY ended_at_ms DESC, session_id DESC LIMIT 1
            """.trimIndent(),
            arrayOf(currentSession.first, sessionId, currentSession.second.toString()),
        ).use { cursor ->
            if (cursor.moveToFirst()) cursor.getString(0) to cursor.getLong(1) else null
        } ?: return
        if (currentSession.second - previous.second > settings.sessionLinkWindowMs) return
        val existingGroup = db.rawQuery(
            "SELECT session_group_id FROM session_group_member WHERE session_id = ?",
            arrayOf(previous.first),
        ).use { cursor -> if (cursor.moveToFirst()) cursor.getString(0) else null }
        val groupId = existingGroup ?: UUID.randomUUID().toString()
        if (existingGroup == null) {
            db.execSQL(
                """
                INSERT INTO session_group(session_group_id, device_history_id, created_at_ms)
                VALUES (?, ?, ?)
                """.trimIndent(),
                arrayOf(groupId, currentSession.first, currentSession.second),
            )
            db.execSQL(
                """
                INSERT INTO session_group_member(session_group_id, session_id, ordinal)
                VALUES (?, ?, 0)
                """.trimIndent(),
                arrayOf(groupId, previous.first),
            )
        }
        val ordinal = android.database.DatabaseUtils.longForQuery(
            db,
            "SELECT COALESCE(MAX(ordinal), -1) + 1 FROM session_group_member WHERE session_group_id = ?",
            arrayOf(groupId),
        )
        db.execSQL(
            """
            INSERT OR IGNORE INTO session_group_member(session_group_id, session_id, ordinal)
            VALUES (?, ?, ?)
            """.trimIndent(),
            arrayOf(groupId, sessionId, ordinal),
        )
    }

    private fun resumePendingDeletes(db: SQLiteDatabase) {
        val pending = db.rawQuery(
            "SELECT session_id, file_name FROM session_catalog WHERE pending_delete = 1",
            null,
        ).use { cursor ->
            buildList {
                while (cursor.moveToNext()) add(cursor.getString(0) to cursor.getString(1))
            }
        }
        pending.forEach { (sessionId, fileName) ->
            val sessionFile = File(root, fileName)
            if (sessionFile.exists() && !SQLiteDatabase.deleteDatabase(sessionFile)) return@forEach
            db.execSQL("DELETE FROM session_catalog WHERE session_id = ?", arrayOf(sessionId))
        }
        removeUndersizedGroups(db)
    }

    private fun removeUndersizedGroups(db: SQLiteDatabase) {
        db.execSQL(
            """
            DELETE FROM session_group
            WHERE session_group_id IN (
              SELECT g.session_group_id FROM session_group g
              LEFT JOIN session_group_member m ON m.session_group_id = g.session_group_id
              GROUP BY g.session_group_id HAVING COUNT(m.session_id) < 2
            )
            """.trimIndent(),
        )
    }

    private fun cleanupStagedExports() {
        val cutoff = System.currentTimeMillis() - 86_400_000
        File(root, "Exports").listFiles()
            ?.filter { it.extension == "csv" || it.extension == "floatview" }
            ?.sortedByDescending(File::lastModified)
            ?.forEachIndexed { index, file ->
                if (index >= 20 || file.lastModified() < cutoff) file.delete()
            }
    }

    private fun writeCsv(db: SQLiteDatabase, output: File) {
        BufferedWriter(OutputStreamWriter(FileOutputStream(output), Charsets.UTF_8), 64 * 1024).use { writer ->
            writer.appendLine(
                "sequence,observed_at_ms,slow_observed_at_ms,elapsed_ms,ride_state," +
                    "package_state,package_mode,footpad_state,stop_condition," +
                    "setpoint_adjustment,wheelslip,darkride,charging,alert_reason," +
                    "firmware_fault,pack_voltage_v,duty_cycle,controller_temp_c," +
                    "motor_temp_c,motor_current_a,battery_current_a,erpm,speed_kph," +
                    "detail_json,extension_json",
            )
            db.rawQuery(
                """
                SELECT sequence, observed_at_ms, slow_observed_at_ms, monotonic_elapsed_ms,
                       ride_state, package_state, package_mode, footpad_state,
                       stop_condition, setpoint_adjustment, wheelslip, darkride, charging,
                       alert_reason, firmware_fault, raw_pack_voltage_v, duty_cycle,
                       controller_temp_c, motor_temp_c, motor_current_a, battery_current_a,
                       erpm, speed_kph, detail_json, extension_json
                FROM sample ORDER BY sequence
                """.trimIndent(),
                null,
            ).use { cursor ->
                while (cursor.moveToNext()) {
                    writer.appendLine(
                        (0 until cursor.columnCount).joinToString(",") { index ->
                            if (cursor.isNull(index)) "" else if (
                                index in setOf(4, 5, 6, 7, 8, 9, 13, 14, 23, 24)
                            ) {
                                csv(cursor.getString(index))
                            } else {
                                cursor.getString(index)
                            }
                        },
                    )
                }
            }
        }
    }

    private fun sessionArchiveManifest(db: SQLiteDatabase, sessionId: String): JSONObject =
        db.rawQuery(
            """
            SELECT session_id, device_history_id, device_snapshot_json, profile_snapshot_json,
                   profile_revision, started_at_ms, ended_at_ms, state, integrity,
                   committed_sequence, sample_count, event_count, requested_idle_ms,
                   effective_idle_ms, requested_riding_ms, effective_riding_ms, close_reason,
                   riding_elapsed_ms, total_elapsed_ms, distance_m, max_speed_mps, max_duty_cycle
            FROM session_catalog WHERE session_id = ? AND pending_delete = 0
            """.trimIndent(),
            arrayOf(sessionId),
        ).use { cursor ->
            if (!cursor.moveToFirst()) throw SessionStoreException.SessionNotFound
            JSONObject()
                .put("sessionId", cursor.getString(0))
                .put("deviceHistoryId", cursor.getString(1))
                .put("deviceSnapshot", JSONObject(cursor.getString(2)))
                .put("boardProfileSnapshot", JSONObject(cursor.getString(3)))
                .put("boardProfileRevision", cursor.getLong(4))
                .put("startedAtMs", cursor.getLong(5))
                .put("endedAtMs", if (cursor.isNull(6)) JSONObject.NULL else cursor.getLong(6))
                .put("state", cursor.getString(7))
                .put("integrity", cursor.getString(8))
                .put("committedSequence", cursor.getLong(9))
                .put("sampleCount", cursor.getLong(10))
                .put("eventCount", cursor.getLong(11))
                .put("requestedIdleMs", cursor.getLong(12))
                .put("effectiveIdleMs", cursor.getLong(13))
                .put("requestedRidingMs", cursor.getLong(14))
                .put("effectiveRidingMs", cursor.getLong(15))
                .put("closeReason", cursor.optionalString(16) ?: JSONObject.NULL)
                .put("ridingElapsedMs", cursor.getLong(17))
                .put("totalElapsedMs", cursor.getLong(18))
                .put("distanceM", cursor.optionalDouble(19) ?: JSONObject.NULL)
                .put("maxSpeedMps", cursor.optionalDouble(20) ?: JSONObject.NULL)
                .put("maxDutyCycle", cursor.optionalDouble(21) ?: JSONObject.NULL)
        }

    private fun csv(value: String): String = "\"${value.replace("\"", "\"\"")}\""

    private fun updateCatalogSummary(record: SessionRecord, samples: List<TelemetrySample>) {
        data class Prior(
            val elapsed: Long?,
            val speed: Double?,
            val riding: Long,
            val distance: Double,
            val maxSpeed: Double?,
            val maxDuty: Double?,
            val count: Long,
        )
        val db = catalog.writableDatabase
        val prior = db.rawQuery(
            """
            SELECT last_sample_elapsed_ms, last_speed_mps, riding_elapsed_ms, distance_m,
                   max_speed_mps, max_duty_cycle, sample_count
            FROM session_catalog WHERE job_id = ?
            """.trimIndent(),
            arrayOf(record.jobId.value),
        ).use { cursor ->
            check(cursor.moveToFirst())
            Prior(
                if (cursor.isNull(0)) null else cursor.getLong(0),
                cursor.optionalDouble(1),
                cursor.getLong(2),
                cursor.optionalDouble(3) ?: 0.0,
                cursor.optionalDouble(4),
                cursor.optionalDouble(5),
                cursor.getLong(6),
            )
        }
        var previousElapsed = prior.elapsed
        var previousSpeed = prior.speed
        var riding = prior.riding
        var distance = prior.distance
        var maxSpeed = prior.maxSpeed
        var maxDuty = prior.maxDuty
        for (sample in samples.sortedBy { it.elapsedMs }) {
            val speed = sample.speedKph?.let { kotlin.math.abs(it) / 3.6 }
            previousElapsed?.let { elapsed ->
                val delta = maxOf(0, sample.elapsedMs - elapsed)
                if (sample.rideState == fail.acheron.floatview.backend.RideState.RIDING) riding += delta
                if (speed != null) {
                    distance += ((previousSpeed ?: speed) + speed) * 0.5 * delta / 1_000.0
                }
            }
            if (speed != null) maxSpeed = maxOf(maxSpeed ?: speed, speed)
            sample.dutyCycle?.let { duty ->
                val absolute = kotlin.math.abs(duty)
                maxDuty = maxOf(maxDuty ?: absolute, absolute)
            }
            previousElapsed = sample.elapsedMs
            previousSpeed = speed
        }
        val last = samples.maxByOrNull { it.elapsedMs } ?: return
        db.execSQL(
            """
            UPDATE session_catalog SET committed_sequence = ?, sample_count = ?,
              riding_elapsed_ms = ?, total_elapsed_ms = ?, distance_m = ?,
              max_speed_mps = ?, max_duty_cycle = ?, last_sample_elapsed_ms = ?,
              last_speed_mps = ? WHERE job_id = ?
            """.trimIndent(),
            arrayOf(
                maxOf(record.committedSequence, samples.maxOf { it.sequence }),
                prior.count + samples.size,
                riding,
                last.elapsedMs,
                distance,
                maxSpeed,
                maxDuty,
                last.elapsedMs,
                previousSpeed,
                record.jobId.value,
            ),
        )
    }

    private fun openSession(sessionId: String): SQLiteDatabase {
        val fileName = catalog.writableDatabase.rawQuery(
            "SELECT file_name FROM session_catalog WHERE session_id = ? AND pending_delete = 0",
            arrayOf(sessionId),
        ).use { cursor ->
            if (!cursor.moveToFirst()) throw SessionStoreException.SessionNotFound
            cursor.getString(0)
        }
        val file = File(root, fileName)
        if (!file.exists()) throw SessionStoreException.SessionNotFound
        return SQLiteDatabase.openDatabase(file.path, null, SQLiteDatabase.OPEN_READWRITE)
    }

    private fun readSeries(
        sessionId: String,
        start: Long,
        end: Long,
        fields: List<String>,
        maxPoints: Int,
        lossless: Boolean,
    ): List<SessionSeriesPoint> {
        val uniqueFields = fields.distinct()
        if (uniqueFields.isEmpty()) throw SessionStoreException.InvalidRange
        val columns = uniqueFields.map { field ->
            SERIES_COLUMNS[field] ?: run {
                if (field.isEmpty() || field.toByteArray(Charsets.UTF_8).size > 64) {
                    throw SessionStoreException.UnsupportedField(field)
                }
                val path = ("$." + JSONObject.quote(field)).replace("'", "''")
                "json_extract(extension_json, '$path')"
            }
        }
        val selected = columns.mapIndexed { index, column ->
            if (lossless) "$column AS value_$index" else "AVG($column) AS value_$index"
        }.joinToString()
        val sql: String
        val arguments: Array<String>
        if (lossless) {
            sql = """
                SELECT monotonic_elapsed_ms, $selected FROM sample
                WHERE monotonic_elapsed_ms BETWEEN ? AND ?
                ORDER BY monotonic_elapsed_ms, sequence LIMIT ?
            """.trimIndent()
            arguments = arrayOf(start.toString(), end.toString(), maxPoints.toString())
        } else {
            val span = maxOf(1, end - start + 1)
            sql = """
                SELECT MIN(monotonic_elapsed_ms), $selected FROM sample
                WHERE monotonic_elapsed_ms BETWEEN ? AND ?
                GROUP BY ((monotonic_elapsed_ms - ?) * ? / ?)
                ORDER BY MIN(monotonic_elapsed_ms) LIMIT ?
            """.trimIndent()
            arguments = arrayOf(
                start.toString(),
                end.toString(),
                start.toString(),
                maxPoints.toString(),
                span.toString(),
                maxPoints.toString(),
            )
        }
        return openSession(sessionId).use { db ->
            db.rawQuery(sql, arguments).use { cursor ->
                buildList {
                    while (cursor.moveToNext()) {
                        add(
                            SessionSeriesPoint(
                                cursor.getLong(0),
                                uniqueFields.mapIndexed { index, field ->
                                    field to cursor.optionalDouble(index + 1)
                                }.toMap(),
                            ),
                        )
                    }
                }
            }
        }
    }

    private fun android.database.Cursor.sessionSummary(): SessionSummaryRecord {
        val snapshot = try {
            JSONObject(getString(2))
        } catch (_: Exception) {
            JSONObject()
        }
        return SessionSummaryRecord(
            sessionId = getString(0),
            deviceHistoryId = getString(1),
            displayName = snapshot.optString("displayName", "Board"),
            startedAt = Instant.ofEpochMilli(getLong(3)),
            endedAt = optionalInstant(4),
            state = getString(5),
            integrity = getString(6),
            sampleCount = getLong(7),
            eventCount = getLong(8),
            ridingElapsedMs = getLong(9),
            totalElapsedMs = getLong(10),
            distanceM = optionalDouble(11),
            maxSpeedMps = optionalDouble(12),
            maxDutyCycle = optionalDouble(13),
        )
    }

    private fun encodeCursor(value: SessionSummaryRecord, field: String): String {
        val number = when (field) {
            "totalElapsedMs" -> value.totalElapsedMs.toDouble()
            "ridingElapsedMs" -> value.ridingElapsedMs.toDouble()
            "distanceM" -> value.distanceM ?: -1.0
            else -> value.startedAt.toEpochMilli().toDouble()
        }
        val raw = "$field:$number:${value.sessionId}"
        return Base64.encodeToString(raw.toByteArray(Charsets.UTF_8), Base64.NO_WRAP)
    }

    private fun decodeCursor(value: String): Triple<String, Double, String> {
        val raw = try {
            String(Base64.decode(value, Base64.DEFAULT), Charsets.UTF_8)
        } catch (_: Exception) {
            throw SessionStoreException.InvalidCursor
        }
        val first = raw.indexOf(':')
        val second = if (first >= 0) raw.indexOf(':', first + 1) else -1
        val field = if (first > 0) raw.substring(0, first) else ""
        val number = if (second > first) raw.substring(first + 1, second).toDoubleOrNull() else null
        val sessionId = if (second >= 0) raw.substring(second + 1) else ""
        if (
            SESSION_SORT_COLUMNS[field] == null ||
            number == null ||
            sessionId.isEmpty()
        ) {
            throw SessionStoreException.InvalidCursor
        }
        return Triple(field, number, sessionId)
    }

    private fun ensureSavedDevice(
        db: SQLiteDatabase,
        deviceId: DeviceId,
        advertisedName: String?,
        at: Instant,
    ): SavedDeviceRecord {
        db.rawQuery(
            """
            SELECT saved_device_id, device_history_id, revision, custom_display_name,
                   advertised_name, opaque_platform_id, last_seen_at_ms, last_connected_at_ms
            FROM saved_device WHERE opaque_platform_id = ? LIMIT 1
            """.trimIndent(),
            arrayOf(deviceId.value),
        ).use { cursor ->
            if (cursor.moveToFirst()) {
                val customName = cursor.optionalString(3)
                val storedAdvertisedName = advertisedName ?: cursor.optionalString(4)
                val identifier = cursor.getString(5)
                val record = SavedDeviceRecord(
                    savedDeviceId = cursor.getString(0),
                    deviceHistoryId = cursor.getString(1),
                    revision = cursor.getInt(2),
                    displayName = resolvedDisplayName(customName, storedAdvertisedName, identifier),
                    customDisplayName = customName,
                    advertisedName = storedAdvertisedName,
                    bluetoothIdentifier = identifier,
                    lastSeenAt = cursor.optionalInstant(6),
                    lastConnectedAt = at,
                    profileComplete = false,
                )
                db.execSQL(
                    """
                    UPDATE saved_device
                    SET last_connected_at_ms = ?, advertised_name = COALESCE(?, advertised_name)
                    WHERE saved_device_id = ?
                    """.trimIndent(),
                    arrayOf(at.toEpochMilli(), advertisedName, record.savedDeviceId),
                )
                boardProfile(db, record.savedDeviceId, at)
                return record
            }
        }
        val savedDeviceId = UUID.randomUUID().toString()
        val deviceHistoryId = UUID.randomUUID().toString()
        db.execSQL(
            """
            INSERT INTO saved_device(
              saved_device_id, device_history_id, opaque_platform_id, advertised_name,
              created_at_ms, last_connected_at_ms
            ) VALUES (?, ?, ?, ?, ?, ?)
            """.trimIndent(),
            arrayOf(
                savedDeviceId,
                deviceHistoryId,
                deviceId.value,
                advertisedName,
                at.toEpochMilli(),
                at.toEpochMilli(),
            ),
        )
        boardProfile(db, savedDeviceId, at)
        return SavedDeviceRecord(
            savedDeviceId,
            deviceHistoryId,
            0,
            resolvedDisplayName(null, advertisedName, deviceId.value),
            advertisedName = advertisedName,
            bluetoothIdentifier = deviceId.value,
            lastConnectedAt = at,
            profileComplete = false,
        )
    }

    private fun storeControllerRoutingHint(
        db: SQLiteDatabase,
        hint: ControllerRoutingHint,
        at: Instant,
    ) {
        db.execSQL(
            """
            INSERT INTO controller_route_hint(saved_device_id, can_id, controller_uuid, updated_at_ms)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(saved_device_id) DO UPDATE SET
              can_id = excluded.can_id,
              controller_uuid = excluded.controller_uuid,
              updated_at_ms = excluded.updated_at_ms
            """.trimIndent(),
            arrayOf(
                hint.savedDeviceId,
                hint.canId,
                hint.controllerUuid?.toHex(),
                at.toEpochMilli(),
            ),
        )
    }

    private fun clearControllerRoutingHint(db: SQLiteDatabase, savedDeviceId: String) {
        db.execSQL(
            "DELETE FROM controller_route_hint WHERE saved_device_id = ?",
            arrayOf(savedDeviceId),
        )
    }

    private fun boardProfile(
        db: SQLiteDatabase,
        savedDeviceId: String,
        at: Instant,
    ): BoardProfileRecord {
        val exists = android.database.DatabaseUtils.longForQuery(
            db,
            "SELECT COUNT(*) FROM saved_device WHERE saved_device_id = ?",
            arrayOf(savedDeviceId),
        ) == 1L
        if (!exists) throw ProductStoreException.SavedDeviceNotFound
        db.execSQL(
            """
            INSERT OR IGNORE INTO board_profile(saved_device_id, revision, json, updated_at_ms)
            VALUES (?, 0, '{}', ?)
            """.trimIndent(),
            arrayOf(savedDeviceId, at.toEpochMilli()),
        )
        return db.rawQuery(
            "SELECT revision, json FROM board_profile WHERE saved_device_id = ?",
            arrayOf(savedDeviceId),
        ).use { cursor ->
            if (!cursor.moveToFirst()) throw ProductStoreException.SavedDeviceNotFound
            val valuesJson = cursor.getString(1)
            BoardProfileRecord(
                savedDeviceId,
                cursor.getInt(0),
                valuesJson,
                missingFields(valuesJson),
            )
        }
    }

    private fun recordProfileChange(profile: BoardProfileRecord, at: Instant) {
        val record = current ?: return
        val db = sessionDb ?: return
        val elapsedMs = maxOf(0, at.toEpochMilli() - record.startedAt.toEpochMilli())
        val details = JSONObject()
            .put("savedDeviceId", profile.savedDeviceId)
            .put("revision", profile.revision)
            .put("values", JSONObject(profile.valuesJson))
            .toString()
        db.execSQL(
            """
            INSERT OR REPLACE INTO session_event(
              sequence, observed_at_ms, monotonic_elapsed_ms, kind, details_json
            ) VALUES (?, ?, ?, 'board_profile_changed', ?)
            """.trimIndent(),
            arrayOf(profile.revision, at.toEpochMilli(), elapsedMs, details),
        )
        catalog.writableDatabase.execSQL(
            """
            UPDATE session_catalog SET event_count = event_count + 1
            WHERE job_id = ? AND state = 'active'
            """.trimIndent(),
            arrayOf(record.jobId.value),
        )
    }

    private fun missingFields(valuesJson: String): List<String> {
        val values = try {
            JSONObject(valuesJson)
        } catch (_: Exception) {
            return INITIAL_MISSING_FIELDS
        }
        return INITIAL_MISSING_FIELDS.filter { key ->
            !values.has(key) || values.isNull(key)
        }
    }

    private fun resolvedDisplayName(
        customDisplayName: String?,
        advertisedName: String?,
        bluetoothIdentifier: String,
    ): String = customDisplayName?.takeIf(String::isNotBlank)
        ?: advertisedName?.takeIf(String::isNotBlank)
        ?: bluetoothIdentifier

    private fun validInitialProfileValues(values: JSONObject): Boolean {
        fun integerInRange(key: String, range: IntRange): Boolean {
            if (!values.has(key) || values.isNull(key)) return true
            val raw = values.get(key)
            if (raw !is Number) return false
            val number = raw.toDouble()
            return number.isFinite() && number % 1.0 == 0.0 && number.toInt() in range
        }
        if (!integerInRange("seriesCount", 1..36)) return false
        if (!integerInRange("parallelCount", 1..20)) return false
        if (!integerInRange("preferredCanId", 0..253)) return false
        if (!values.has("motor") || values.isNull("motor")) return true
        val motor = values.get("motor") as? String ?: return false
        return motor.isNotEmpty() && motor.toByteArray().size <= 128
    }

    private fun preferredCanId(values: JSONObject): Int? =
        if (!values.has("preferredCanId") || values.isNull("preferredCanId")) {
            null
        } else {
            values.optInt("preferredCanId")
        }

    private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }

    private fun String.hexToBytes(): ByteArray? {
        if (length != 24) return null
        return try {
            ByteArray(12) { index ->
                substring(index * 2, index * 2 + 2).toInt(16).toByte()
            }
        } catch (_: NumberFormatException) {
            null
        }
    }

    private fun asset(name: String): String =
        appContext.assets.open(name).bufferedReader().use { it.readText() }

    private fun executePragma(db: SQLiteDatabase, statement: String) {
        db.rawQuery(statement, null).use { cursor ->
            while (cursor.moveToNext()) {
                // Consume result rows so SQLite completes query-style PRAGMAs
                // before the database is used or closed.
            }
        }
    }

    private fun executeScript(
        db: SQLiteDatabase,
        script: String,
        skipPragmas: Boolean = false,
    ) {
        script.lineSequence()
            .filterNot { it.trimStart().startsWith("--") }
            .filterNot {
                skipPragmas && it.trimStart().startsWith("PRAGMA", ignoreCase = true)
            }
            .joinToString("\n")
            .split(';')
            .map(String::trim)
            .filter(String::isNotEmpty)
            .forEach(db::execSQL)
    }

    private fun telemetryDetailJson(sample: TelemetrySample): String {
        val value = JSONObject()
        fun put(key: String, number: Double?) {
            if (number != null) {
                require(number.isFinite()) { "Telemetry detail must be finite" }
                value.put(key, number)
            }
        }
        put("pitchDeg", sample.pitchDeg)
        put("balancePitchDeg", sample.balancePitchDeg)
        put("rollDeg", sample.rollDeg)
        put("directionalMotorCurrentA", sample.directionalMotorCurrentA)
        put("filteredMotorCurrentA", sample.filteredMotorCurrentA)
        put("requestedBalanceCurrentA", sample.requestedBalanceCurrentA)
        put("boosterCurrentA", sample.boosterCurrentA)
        put("fieldWeakeningCurrentA", sample.fieldWeakeningCurrentA)
        put("footpadAdc1V", sample.footpadAdc1V)
        put("footpadAdc2V", sample.footpadAdc2V)
        put("remoteInput", sample.remoteInput)
        put("setpointDeg", sample.setpointDeg)
        put("atrSetpointDeg", sample.atrSetpointDeg)
        put("brakeTiltSetpointDeg", sample.brakeTiltSetpointDeg)
        put("torqueTiltSetpointDeg", sample.torqueTiltSetpointDeg)
        put("turnTiltSetpointDeg", sample.turnTiltSetpointDeg)
        put("remoteSetpointDeg", sample.remoteSetpointDeg)
        put(
            "atrAccelerationDifferenceErpmPerSecond",
            sample.atrAccelerationDifferenceErpmPerSecond,
        )
        put("atrSpeedBoost", sample.atrSpeedBoost)
        put("distanceM", sample.distanceM)
        put("odometerM", sample.odometerM)
        put("ampHoursDrawnAh", sample.ampHoursDrawnAh)
        put("ampHoursChargedAh", sample.ampHoursChargedAh)
        put("wattHoursDrawnWh", sample.wattHoursDrawnWh)
        put("wattHoursChargedWh", sample.wattHoursChargedWh)
        put("batteryLevel", sample.batteryLevel)
        put("chargingCurrentA", sample.chargingCurrentA)
        put("chargingVoltageV", sample.chargingVoltageV)
        return value.toString().also {
            require(it.toByteArray(Charsets.UTF_8).size <= 16_384) {
                "Telemetry detail JSON exceeds its bound"
            }
        }
    }

    private fun telemetryExtensionJson(values: Map<String, Double>): String {
        require(values.size <= 32) { "Too many telemetry extension fields" }
        values.forEach { (key, value) ->
            require(key.isNotEmpty() && key.toByteArray(Charsets.UTF_8).size <= 64) {
                "Telemetry extension key exceeds its bound"
            }
            require(value.isFinite()) { "Telemetry extension value must be finite" }
        }
        return JSONObject(values).toString().also {
            require(it.toByteArray(Charsets.UTF_8).size <= 16_384) {
                "Telemetry extension JSON exceeds its bound"
            }
        }
    }

    private fun transitionValues(sample: TelemetrySample): List<Pair<String, String>> = buildList {
        sample.packageState?.let { add("packageState" to it.wireValue) }
        sample.packageMode?.let { add("packageMode" to it.wireValue) }
        sample.footpadState?.let { add("footpadState" to it.wireValue) }
        sample.stopCondition?.let { add("stopCondition" to it.wireValue) }
        sample.setpointAdjustment?.let { add("setpointAdjustment" to it.wireValue) }
        sample.wheelslip?.let { add("wheelslip" to if (it) "active" else "cleared") }
        sample.darkride?.let { add("darkride" to if (it) "active" else "cleared") }
        sample.charging?.let { add("charging" to if (it) "active" else "cleared") }
        add("alertReason" to (sample.alertReason?.wireValue ?: "none"))
        add("firmwareFault" to (sample.firmwareFault?.wireValue ?: "none"))
    }

    private fun latestTransitionValues(db: SQLiteDatabase): Map<String, String> =
        db.rawQuery(
            """
            SELECT package_state, package_mode, footpad_state, stop_condition,
                   setpoint_adjustment, wheelslip, darkride, charging, alert_reason,
                   firmware_fault FROM sample ORDER BY sequence DESC LIMIT 1
            """.trimIndent(),
            null,
        ).use { cursor ->
            if (!cursor.moveToFirst()) return@use emptyMap()
            buildMap {
                listOf(
                    0 to "packageState",
                    1 to "packageMode",
                    2 to "footpadState",
                    3 to "stopCondition",
                    4 to "setpointAdjustment",
                ).forEach { (index, key) ->
                    if (!cursor.isNull(index)) put(key, cursor.getString(index))
                }
                listOf(5 to "wheelslip", 6 to "darkride", 7 to "charging")
                    .forEach { (index, key) ->
                        if (!cursor.isNull(index)) {
                            put(key, if (cursor.getInt(index) == 0) "cleared" else "active")
                        }
                    }
                put("alertReason", if (cursor.isNull(8)) "none" else cursor.getString(8))
                put("firmwareFault", if (cursor.isNull(9)) "none" else cursor.getString(9))
            }
        }

    private class CatalogOpenHelper(private val appContext: Context, path: String) :
        SQLiteOpenHelper(appContext, path, null, 1) {
        init {
            setWriteAheadLoggingEnabled(true)
        }

        override fun onConfigure(db: SQLiteDatabase) {
            db.setForeignKeyConstraintsEnabled(true)
            db.execSQL("PRAGMA synchronous = FULL")
        }
        override fun onCreate(db: SQLiteDatabase) {
            val script = appContext.assets.open("catalog.sql").bufferedReader().use { it.readText() }
            script.lineSequence()
                .filterNot { it.trimStart().startsWith("--") }
                // SQLiteOpenHelper wraps onCreate in a transaction. WAL and
                // synchronous mode are configured above, outside this script.
                .filterNot { it.trimStart().startsWith("PRAGMA", ignoreCase = true) }
                .joinToString("\n")
                .split(';')
                .map(String::trim)
                .filter(String::isNotEmpty)
                .forEach(db::execSQL)
        }
        override fun onUpgrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) {
            error("Storage version marker did not reset an outdated catalog")
        }

        override fun onDowngrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) {
            error("Storage version marker did not reset an outdated catalog")
        }
    }

    private fun android.database.Cursor.optionalString(index: Int): String? =
        if (isNull(index)) null else getString(index)

    private fun android.database.Cursor.optionalInstant(index: Int): Instant? =
        if (isNull(index)) null else Instant.ofEpochMilli(getLong(index))

    private fun android.database.Cursor.optionalDouble(index: Int): Double? =
        if (isNull(index)) null else getDouble(index)

    private companion object {
        val INITIAL_MISSING_FIELDS = listOf("seriesCount", "parallelCount", "motor")
        val SETTING_KEYS = setOf(
            "idleProbeIntervalMs",
            "idlePersistIntervalMs",
            "ridingPersistIntervalMs",
            "postRideTailMs",
            "reconnectDurationMs",
            "automaticSessionLinkingEnabled",
            "sessionLinkWindowMs",
            "themePreference",
            "unitPreference",
        )
        const val MAX_PROFILE_BYTES = 65_536
        val SERIES_COLUMNS = mapOf(
            "packVoltageV" to "raw_pack_voltage_v",
            "dutyCycle" to "duty_cycle",
            "controllerTempC" to "controller_temp_c",
            "motorTempC" to "motor_temp_c",
            "motorCurrentA" to "motor_current_a",
            "batteryCurrentA" to "battery_current_a",
            "erpm" to "erpm",
            "speedKph" to "speed_kph",
            "pitchDeg" to "json_extract(detail_json, '$.pitchDeg')",
            "balancePitchDeg" to "json_extract(detail_json, '$.balancePitchDeg')",
            "rollDeg" to "json_extract(detail_json, '$.rollDeg')",
            "directionalMotorCurrentA" to
                "json_extract(detail_json, '$.directionalMotorCurrentA')",
            "filteredMotorCurrentA" to "json_extract(detail_json, '$.filteredMotorCurrentA')",
            "requestedBalanceCurrentA" to
                "json_extract(detail_json, '$.requestedBalanceCurrentA')",
            "boosterCurrentA" to "json_extract(detail_json, '$.boosterCurrentA')",
            "fieldWeakeningCurrentA" to
                "json_extract(detail_json, '$.fieldWeakeningCurrentA')",
            "footpadAdc1V" to "json_extract(detail_json, '$.footpadAdc1V')",
            "footpadAdc2V" to "json_extract(detail_json, '$.footpadAdc2V')",
            "remoteInput" to "json_extract(detail_json, '$.remoteInput')",
            "setpointDeg" to "json_extract(detail_json, '$.setpointDeg')",
            "atrSetpointDeg" to "json_extract(detail_json, '$.atrSetpointDeg')",
            "brakeTiltSetpointDeg" to
                "json_extract(detail_json, '$.brakeTiltSetpointDeg')",
            "torqueTiltSetpointDeg" to
                "json_extract(detail_json, '$.torqueTiltSetpointDeg')",
            "turnTiltSetpointDeg" to "json_extract(detail_json, '$.turnTiltSetpointDeg')",
            "remoteSetpointDeg" to "json_extract(detail_json, '$.remoteSetpointDeg')",
            "atrAccelerationDifferenceErpmPerSecond" to
                "json_extract(detail_json, '$.atrAccelerationDifferenceErpmPerSecond')",
            "atrSpeedBoost" to "json_extract(detail_json, '$.atrSpeedBoost')",
            "distanceM" to "json_extract(detail_json, '$.distanceM')",
            "odometerM" to "json_extract(detail_json, '$.odometerM')",
            "ampHoursDrawnAh" to "json_extract(detail_json, '$.ampHoursDrawnAh')",
            "ampHoursChargedAh" to "json_extract(detail_json, '$.ampHoursChargedAh')",
            "wattHoursDrawnWh" to "json_extract(detail_json, '$.wattHoursDrawnWh')",
            "wattHoursChargedWh" to
                "json_extract(detail_json, '$.wattHoursChargedWh')",
            "batteryLevel" to "json_extract(detail_json, '$.batteryLevel')",
            "chargingCurrentA" to "json_extract(detail_json, '$.chargingCurrentA')",
            "chargingVoltageV" to "json_extract(detail_json, '$.chargingVoltageV')",
        )
        val SESSION_SORT_COLUMNS = mapOf(
            "startedAt" to "started_at_ms",
            "totalElapsedMs" to "total_elapsed_ms",
            "ridingElapsedMs" to "riding_elapsed_ms",
            "distanceM" to "COALESCE(distance_m, -1)",
        )
    }
}
