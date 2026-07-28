package fail.acheron.floatview.logging

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import fail.acheron.floatview.core.DeviceId
import fail.acheron.floatview.core.JobId
import fail.acheron.floatview.core.LogStorePort
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
) : LogStorePort {
    private val appContext = context.applicationContext
    private val root = File(appContext.filesDir, "float-view/logs").apply { mkdirs() }
    private val catalog = CatalogOpenHelper(appContext, File(root, "catalog.sqlite").path)
    private var current: SessionRecord? = null
    private var sessionDb: SQLiteDatabase? = null

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
            catalogDb.execSQL(
                """
                INSERT OR IGNORE INTO saved_device(
                  saved_device_id, device_history_id, opaque_platform_id, display_name, created_at_ms
                ) VALUES (?, ?, ?, ?, ?)
                """.trimIndent(),
                arrayOf("saved-${deviceId.value}", "history-${deviceId.value}", deviceId.value, "Board", started),
            )
            catalogDb.execSQL(
                """
                INSERT INTO session_catalog(
                  session_id, job_id, device_history_id, device_snapshot_json,
                  profile_snapshot_json, profile_revision, file_name, started_at_ms,
                  state, integrity, requested_idle_ms, effective_idle_ms,
                  requested_riding_ms, effective_riding_ms
                ) VALUES (?, ?, ?, '{}', '{}', 0, ?, ?, 'active', 'complete', 1000, 1000, 100, 100)
                """.trimIndent(),
                arrayOf(sessionId.value, jobId.value, "history-${deviceId.value}", fileName, started),
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
}
