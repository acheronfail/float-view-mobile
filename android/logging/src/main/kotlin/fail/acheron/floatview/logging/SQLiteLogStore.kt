package fail.acheron.floatview.logging

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import fail.acheron.floatview.core.DeviceId
import fail.acheron.floatview.core.BoardProfileRecord
import fail.acheron.floatview.core.JobId
import fail.acheron.floatview.core.LogStorePort
import fail.acheron.floatview.core.ProductStoreException
import fail.acheron.floatview.core.ProductStorePort
import fail.acheron.floatview.core.SavedDeviceRecord
import fail.acheron.floatview.core.SessionId
import fail.acheron.floatview.core.SessionEventRecord
import fail.acheron.floatview.core.SessionListQuery
import fail.acheron.floatview.core.SessionPage
import fail.acheron.floatview.core.SessionRecord
import fail.acheron.floatview.core.SessionSeriesPoint
import fail.acheron.floatview.core.SessionStoreException
import fail.acheron.floatview.core.SessionStorePort
import fail.acheron.floatview.core.SessionSummaryRecord
import fail.acheron.floatview.core.TelemetrySample
import android.util.Base64
import org.json.JSONObject
import java.io.File
import java.time.Instant
import java.util.UUID

/**
 * Production durable store. The backend owns this object; no Activity or WebView
 * reference is retained. Catalog and session commits are bounded independently.
 */
class SQLiteLogStore(
    context: Context,
    private val availableBytes: () -> Long = { context.filesDir.usableSpace },
) : LogStorePort, ProductStorePort, SessionStorePort {
    private val appContext = context.applicationContext
    private val root = File(appContext.filesDir, "float-view/logs").apply { mkdirs() }
    private val catalog = CatalogOpenHelper(appContext, File(root, "catalog.sqlite").path)
    private var current: SessionRecord? = null
    private var sessionDb: SQLiteDatabase? = null

    init {
        backfillV2Summaries(catalog.writableDatabase)
        migrateLegacySavedDeviceIdentifiers(catalog.writableDatabase)
    }

    @Synchronized
    override fun create(deviceId: DeviceId, at: Instant): SessionRecord {
        check(StoragePolicy.level(availableBytes()) != StorageLevel.STOP) {
            "Storage below durable logging stop threshold"
        }
        current?.let { return it }
        val sessionId = SessionId(UUID.randomUUID().toString())
        val jobId = JobId(UUID.randomUUID().toString())
        val fileName = "session-${sessionId.value}.sqlite"
        val sessionFile = File(root, fileName)
        // Force catalog creation before creating the per-session file so a
        // catalog migration/configuration failure cannot leave an orphan.
        val catalogDb = catalog.writableDatabase
        val db = SQLiteDatabase.openOrCreateDatabase(sessionFile, null)
        try {
            check(db.enableWriteAheadLogging()) {
                "Unable to enable WAL for durable session logging"
            }
            executePragma(db, "PRAGMA synchronous = FULL")
            executeScript(db, asset("session-v1.sql"), skipPragmas = true)
            executeScript(db, asset("session-v2.sql"))
        } catch (error: Exception) {
            db.close()
            SQLiteDatabase.deleteDatabase(sessionFile)
            throw error
        }
        val started = at.toEpochMilli()
        catalogDb.beginTransaction()
        try {
            val savedDevice = ensureSavedDevice(catalogDb, deviceId, at)
            val profile = boardProfile(catalogDb, savedDevice.savedDeviceId, at)
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
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'active', 'complete', 1000, 1000, 100, 100)
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

    @Synchronized
    override fun append(jobId: JobId, samples: List<TelemetrySample>) {
        val record = requireNotNull(current) { "No active session" }
        require(record.jobId == jobId) { "Wrong logging job" }
        if (samples.isEmpty()) return
        check(StoragePolicy.level(availableBytes()) != StorageLevel.STOP) {
            "Storage below durable logging stop threshold"
        }
        val db = requireNotNull(sessionDb)
        db.beginTransaction()
        try {
            for (sample in samples) {
                db.execSQL(
                    """
                    INSERT OR IGNORE INTO sample(
                      sequence, observed_at_ms, monotonic_elapsed_ms, ride_state,
                      raw_pack_voltage_v, duty_cycle, controller_temp_c, motor_temp_c,
                      footpad_state, extension_json, motor_current_a, battery_current_a,
                      erpm, speed_kph
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """.trimIndent(),
                    arrayOf(
                        sample.sequence, sample.observedAt.toEpochMilli(), sample.elapsedMs,
                        sample.rideState.name.lowercase(), sample.packVoltageV, sample.dutyCycle,
                        sample.controllerTempC, sample.motorTempC, sample.footpadState,
                        JSONObject(sample.extension).toString(),
                        sample.extension["motorCurrentA"],
                        sample.extension["batteryCurrentA"],
                        sample.extension["erpm"],
                        sample.extension["speedKph"],
                    ),
                )
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
    }

    @Synchronized
    override fun close(jobId: JobId, at: Instant, reason: String): SessionRecord {
        val record = requireNotNull(current) { "No active session" }
        require(record.jobId == jobId) { "Wrong logging job" }
        sessionDb?.let { executePragma(it, "PRAGMA wal_checkpoint(FULL)") }
        sessionDb?.close()
        sessionDb = null
        catalog.writableDatabase.execSQL(
            """
            UPDATE session_catalog
            SET ended_at_ms = ?, state = 'completed', committed_sequence = ?, close_reason = ?
            WHERE job_id = ? AND state = 'active'
            """.trimIndent(),
            arrayOf(at.toEpochMilli(), record.committedSequence, reason, jobId.value),
        )
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
        val conditions = mutableListOf<String>()
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
            FROM session_catalog WHERE session_id = ?
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
        if (startElapsedMs > endElapsedMs || maxPoints !in 1..10_000) {
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
            10_000,
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
                SELECT sequence, monotonic_elapsed_ms, kind, details_json
                FROM session_event WHERE monotonic_elapsed_ms BETWEEN ? AND ?
                ORDER BY monotonic_elapsed_ms, sequence LIMIT 10000
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
    override fun savedDevices(): List<SavedDeviceRecord> {
        val db = catalog.writableDatabase
        return db.rawQuery(
            """
            SELECT d.saved_device_id, d.device_history_id, d.display_name,
                   d.advertised_name, d.last_seen_at_ms, d.last_connected_at_ms,
                   COALESCE(p.json, '{}')
            FROM saved_device d
            LEFT JOIN board_profile p ON p.saved_device_id = d.saved_device_id
            ORDER BY COALESCE(d.last_connected_at_ms, d.created_at_ms) DESC
            """.trimIndent(),
            null,
        ).use { cursor ->
            buildList {
                while (cursor.moveToNext()) {
                    val valuesJson = cursor.getString(6)
                    add(
                        SavedDeviceRecord(
                            savedDeviceId = cursor.getString(0),
                            deviceHistoryId = cursor.getString(1),
                            displayName = cursor.getString(2),
                            advertisedName = cursor.optionalString(3),
                            lastSeenAt = cursor.optionalInstant(4),
                            lastConnectedAt = cursor.optionalInstant(5),
                            profileComplete = missingFields(valuesJson).isEmpty(),
                        ),
                    )
                }
            }
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
            patch.keys().forEach { key -> values.put(key, patch.get(key)) }
            if (!validInitialProfileValues(values)) {
                throw ProductStoreException.InvalidProfilePatch
            }
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
            val speed = sample.extension["speedKph"]?.let { kotlin.math.abs(it) / 3.6 }
            previousElapsed?.let { elapsed ->
                val delta = maxOf(0, sample.elapsedMs - elapsed)
                if (sample.rideState == fail.acheron.floatview.core.RideState.RIDING) riding += delta
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

    private fun backfillV2Summaries(catalogDb: SQLiteDatabase) {
        val completed = catalogDb.rawQuery(
            """
            SELECT CAST(value AS INTEGER) FROM schema_metadata
            WHERE key = 'summary_backfill_version'
            """.trimIndent(),
            null,
        ).use { cursor -> cursor.moveToFirst() && cursor.getInt(0) == 2 }
        if (completed) return
        val files = catalogDb.rawQuery(
            "SELECT session_id, file_name FROM session_catalog",
            null,
        ).use { cursor ->
            buildList {
                while (cursor.moveToNext()) add(cursor.getString(0) to cursor.getString(1))
            }
        }
        for ((sessionId, fileName) in files) {
            val file = File(root, fileName)
            if (!file.exists()) continue
            val db = SQLiteDatabase.openDatabase(file.path, null, SQLiteDatabase.OPEN_READWRITE)
            try {
                val version = db.rawQuery(
                    """
                    SELECT CAST(value AS INTEGER) FROM schema_metadata
                    WHERE key = 'session_version'
                    """.trimIndent(),
                    null,
                ).use { cursor -> if (cursor.moveToFirst()) cursor.getInt(0) else 1 }
                if (version < LogSchema.SESSION_VERSION) {
                    executeScript(db, asset("session-v2.sql"))
                }
                var committedSequence = 0L
                var count = 0L
                var lastElapsed: Long? = null
                var previousSpeed: Double? = null
                var ridingElapsed = 0L
                var distance = 0.0
                var maxSpeed: Double? = null
                var maxDuty: Double? = null
                db.rawQuery(
                    """
                    SELECT sequence, monotonic_elapsed_ms, ride_state, duty_cycle, extension_json
                    FROM sample ORDER BY monotonic_elapsed_ms, sequence
                    """.trimIndent(),
                    null,
                ).use { cursor ->
                    while (cursor.moveToNext()) {
                        committedSequence = cursor.getLong(0)
                        val elapsed = cursor.getLong(1)
                        val riding = cursor.getString(2) == "riding"
                        val duty = cursor.optionalDouble(3)?.let { kotlin.math.abs(it) }
                        val speed = try {
                            JSONObject(cursor.getString(4))
                                .takeIf { it.has("speedKph") && !it.isNull("speedKph") }
                                ?.getDouble("speedKph")
                                ?.let { kotlin.math.abs(it) / 3.6 }
                        } catch (_: Exception) {
                            null
                        }
                        lastElapsed?.let { previousElapsed ->
                            val delta = maxOf(0, elapsed - previousElapsed)
                            if (riding) ridingElapsed += delta
                            if (speed != null) {
                                distance +=
                                    ((previousSpeed ?: speed) + speed) * 0.5 * delta / 1_000.0
                            }
                        }
                        if (speed != null) maxSpeed = maxOf(maxSpeed ?: speed, speed)
                        if (duty != null) maxDuty = maxOf(maxDuty ?: duty, duty)
                        lastElapsed = elapsed
                        previousSpeed = speed
                        count += 1
                    }
                }
                catalogDb.execSQL(
                    """
                    UPDATE session_catalog SET committed_sequence = ?, sample_count = ?,
                      riding_elapsed_ms = ?, total_elapsed_ms = ?, distance_m = ?,
                      max_speed_mps = ?, max_duty_cycle = ?, last_sample_elapsed_ms = ?,
                      last_speed_mps = ? WHERE session_id = ?
                    """.trimIndent(),
                    arrayOf(
                        committedSequence,
                        count,
                        ridingElapsed,
                        lastElapsed ?: 0,
                        distance,
                        maxSpeed,
                        maxDuty,
                        lastElapsed,
                        previousSpeed,
                        sessionId,
                    ),
                )
            } finally {
                db.close()
            }
        }
        catalogDb.execSQL(
            """
            INSERT OR REPLACE INTO schema_metadata(key, value)
            VALUES ('summary_backfill_version', '2')
            """.trimIndent(),
        )
    }

    private fun openSession(sessionId: String): SQLiteDatabase {
        val fileName = catalog.writableDatabase.rawQuery(
            "SELECT file_name FROM session_catalog WHERE session_id = ?",
            arrayOf(sessionId),
        ).use { cursor ->
            if (!cursor.moveToFirst()) throw SessionStoreException.SessionNotFound
            cursor.getString(0)
        }
        val file = File(root, fileName)
        if (!file.exists()) throw SessionStoreException.SessionNotFound
        val db = SQLiteDatabase.openDatabase(file.path, null, SQLiteDatabase.OPEN_READWRITE)
        val version = db.rawQuery(
            "SELECT CAST(value AS INTEGER) FROM schema_metadata WHERE key = 'session_version'",
            null,
        ).use { cursor -> if (cursor.moveToFirst()) cursor.getInt(0) else 1 }
        if (version < LogSchema.SESSION_VERSION) executeScript(db, asset("session-v2.sql"))
        return db
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
            SERIES_COLUMNS[field] ?: throw SessionStoreException.UnsupportedField(field)
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
        at: Instant,
    ): SavedDeviceRecord {
        db.rawQuery(
            """
            SELECT saved_device_id, device_history_id, display_name, advertised_name,
                   last_seen_at_ms, last_connected_at_ms
            FROM saved_device WHERE opaque_platform_id = ? LIMIT 1
            """.trimIndent(),
            arrayOf(deviceId.value),
        ).use { cursor ->
            if (cursor.moveToFirst()) {
                val record = SavedDeviceRecord(
                    savedDeviceId = cursor.getString(0),
                    deviceHistoryId = cursor.getString(1),
                    displayName = cursor.getString(2),
                    advertisedName = cursor.optionalString(3),
                    lastSeenAt = cursor.optionalInstant(4),
                    lastConnectedAt = at,
                    profileComplete = false,
                )
                db.execSQL(
                    "UPDATE saved_device SET last_connected_at_ms = ? WHERE saved_device_id = ?",
                    arrayOf(at.toEpochMilli(), record.savedDeviceId),
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
              saved_device_id, device_history_id, opaque_platform_id, display_name,
              created_at_ms, last_connected_at_ms
            ) VALUES (?, ?, ?, 'Board', ?, ?)
            """.trimIndent(),
            arrayOf(
                savedDeviceId,
                deviceHistoryId,
                deviceId.value,
                at.toEpochMilli(),
                at.toEpochMilli(),
            ),
        )
        boardProfile(db, savedDeviceId, at)
        return SavedDeviceRecord(
            savedDeviceId,
            deviceHistoryId,
            "Board",
            lastConnectedAt = at,
            profileComplete = false,
        )
    }

    private fun migrateLegacySavedDeviceIdentifiers(db: SQLiteDatabase) {
        val legacy = db.rawQuery(
            """
            SELECT saved_device_id, device_history_id
            FROM saved_device
            WHERE saved_device_id = 'saved-' || opaque_platform_id
               OR device_history_id = 'history-' || opaque_platform_id
            """.trimIndent(),
            null,
        ).use { cursor ->
            buildList {
                while (cursor.moveToNext()) add(cursor.getString(0) to cursor.getString(1))
            }
        }
        if (legacy.isEmpty()) return
        db.beginTransaction()
        try {
            db.execSQL("PRAGMA defer_foreign_keys = ON")
            legacy.forEach { (oldSavedDeviceId, oldDeviceHistoryId) ->
                val savedDeviceId = UUID.randomUUID().toString()
                val deviceHistoryId = UUID.randomUUID().toString()
                val snapshots = db.rawQuery(
                    """
                    SELECT session_id, device_snapshot_json FROM session_catalog
                    WHERE device_history_id = ?
                    """.trimIndent(),
                    arrayOf(oldDeviceHistoryId),
                ).use { cursor ->
                    buildList {
                        while (cursor.moveToNext()) add(cursor.getString(0) to cursor.getString(1))
                    }
                }
                db.execSQL(
                    """
                    UPDATE saved_device SET saved_device_id = ?, device_history_id = ?
                    WHERE saved_device_id = ?
                    """.trimIndent(),
                    arrayOf(savedDeviceId, deviceHistoryId, oldSavedDeviceId),
                )
                db.execSQL(
                    "UPDATE board_profile SET saved_device_id = ? WHERE saved_device_id = ?",
                    arrayOf(savedDeviceId, oldSavedDeviceId),
                )
                db.execSQL(
                    "UPDATE session_catalog SET device_history_id = ? WHERE device_history_id = ?",
                    arrayOf(deviceHistoryId, oldDeviceHistoryId),
                )
                db.execSQL(
                    "UPDATE session_group SET device_history_id = ? WHERE device_history_id = ?",
                    arrayOf(deviceHistoryId, oldDeviceHistoryId),
                )
                snapshots.forEach { (sessionId, snapshotJson) ->
                    val snapshot = JSONObject(snapshotJson)
                        .put("savedDeviceId", savedDeviceId)
                        .put("deviceHistoryId", deviceHistoryId)
                    db.execSQL(
                        """
                        UPDATE session_catalog SET device_snapshot_json = ?
                        WHERE session_id = ?
                        """.trimIndent(),
                        arrayOf(snapshot.toString(), sessionId),
                    )
                }
            }
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
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
        if (!values.has("motor") || values.isNull("motor")) return true
        val motor = values.get("motor") as? String ?: return false
        return motor.isNotEmpty() && motor.toByteArray().size <= 128
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

    private class CatalogOpenHelper(private val appContext: Context, path: String) :
        SQLiteOpenHelper(appContext, path, null, LogSchema.CATALOG_VERSION) {
        init {
            setWriteAheadLoggingEnabled(true)
        }

        override fun onConfigure(db: SQLiteDatabase) {
            db.setForeignKeyConstraintsEnabled(true)
            db.execSQL("PRAGMA synchronous = FULL")
        }
        override fun onCreate(db: SQLiteDatabase) {
            val script = appContext.assets.open("catalog-v1.sql").bufferedReader().use { it.readText() }
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
            val migration = appContext.assets.open("catalog-v2.sql")
                .bufferedReader().use { it.readText() }
            migration.lineSequence()
                .filterNot { it.trimStart().startsWith("--") }
                .joinToString("\n")
                .split(';')
                .map(String::trim)
                .filter(String::isNotEmpty)
                .forEach(db::execSQL)
        }
        override fun onUpgrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) {
            if (oldVersion < 2 && newVersion >= 2) {
                val script = appContext.assets.open("catalog-v2.sql")
                    .bufferedReader().use { it.readText() }
                script.lineSequence()
                    .filterNot { it.trimStart().startsWith("--") }
                    .joinToString("\n")
                    .split(';')
                    .map(String::trim)
                    .filter(String::isNotEmpty)
                    .forEach(db::execSQL)
            }
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
        const val MAX_PROFILE_BYTES = 65_536
        val SERIES_COLUMNS = mapOf(
            "packVoltageV" to "raw_pack_voltage_v",
            "dutyCycle" to "duty_cycle",
            "controllerTempC" to "controller_temp_c",
            "motorTempC" to "motor_temp_c",
            "footpadState" to "footpad_state",
            "motorCurrentA" to "motor_current_a",
            "batteryCurrentA" to "battery_current_a",
            "erpm" to "erpm",
            "speedKph" to "speed_kph",
        )
        val SESSION_SORT_COLUMNS = mapOf(
            "startedAt" to "started_at_ms",
            "totalElapsedMs" to "total_elapsed_ms",
            "ridingElapsedMs" to "riding_elapsed_ms",
            "distanceM" to "COALESCE(distance_m, -1)",
        )
    }
}
