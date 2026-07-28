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
import fail.acheron.floatview.core.SessionRecord
import fail.acheron.floatview.core.TelemetrySample
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
) : LogStorePort, ProductStorePort {
    private val appContext = context.applicationContext
    private val root = File(appContext.filesDir, "float-view/logs").apply { mkdirs() }
    private val catalog = CatalogOpenHelper(appContext, File(root, "catalog.sqlite").path)
    private var current: SessionRecord? = null
    private var sessionDb: SQLiteDatabase? = null

    init {
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
                      footpad_state, extension_json
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """.trimIndent(),
                    arrayOf(
                        sample.sequence, sample.observedAt.toEpochMilli(), sample.elapsedMs,
                        sample.rideState.name.lowercase(), sample.packVoltageV, sample.dutyCycle,
                        sample.controllerTempC, sample.motorTempC, sample.footpadState,
                        JSONObject(sample.extension).toString(),
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
        }
        override fun onUpgrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) {
            error("No catalog migration from $oldVersion to $newVersion")
        }
    }

    private fun android.database.Cursor.optionalString(index: Int): String? =
        if (isNull(index)) null else getString(index)

    private fun android.database.Cursor.optionalInstant(index: Int): Instant? =
        if (isNull(index)) null else Instant.ofEpochMilli(getLong(index))

    private companion object {
        val INITIAL_MISSING_FIELDS = listOf("seriesCount", "parallelCount", "motor")
        const val MAX_PROFILE_BYTES = 65_536
    }
}
