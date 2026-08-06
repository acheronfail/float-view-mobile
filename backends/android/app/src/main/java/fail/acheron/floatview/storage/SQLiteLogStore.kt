package fail.acheron.floatview.storage

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import android.database.DatabaseUtils
import android.os.CancellationSignal
import fail.acheron.floatview.backend.DeviceId
import fail.acheron.floatview.backend.BoardProfileRecord
import fail.acheron.floatview.backend.BoardHistorySummaryRecord
import fail.acheron.floatview.backend.ControllerRoutingHint
import fail.acheron.floatview.backend.JobId
import fail.acheron.floatview.backend.LogStorePort
import fail.acheron.floatview.backend.LocationObservation
import fail.acheron.floatview.backend.ProductStoreException
import fail.acheron.floatview.backend.ProductStorePort
import fail.acheron.floatview.backend.SavedDeviceRecord
import fail.acheron.floatview.backend.SessionId
import fail.acheron.floatview.backend.SessionEventRecord
import fail.acheron.floatview.backend.SessionElapsedRange
import fail.acheron.floatview.backend.SessionExportRecord
import fail.acheron.floatview.backend.SessionGapPolicyRecord
import fail.acheron.floatview.backend.SessionListQuery
import fail.acheron.floatview.backend.SessionPage
import fail.acheron.floatview.backend.SessionRecord
import fail.acheron.floatview.backend.SessionSeriesPoint
import fail.acheron.floatview.backend.SessionSeriesReductionRecord
import fail.acheron.floatview.backend.SessionPublishedSnapshotRecord
import fail.acheron.floatview.backend.SessionLocationGapRecord
import fail.acheron.floatview.backend.SessionLocationPageRecord
import fail.acheron.floatview.backend.SessionLocationPointRecord
import fail.acheron.floatview.backend.SessionHistoryConfiguration
import fail.acheron.floatview.backend.SessionHistoryReducer
import fail.acheron.floatview.backend.SessionPruneMetadataRecord
import fail.acheron.floatview.backend.SessionPrunePreviewRecord
import fail.acheron.floatview.backend.SessionMaintenanceJobRecord
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
import java.util.concurrent.Semaphore
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
    private val historicalReadSlots = Semaphore(2)
    private val leaseLock = Any()
    private val activeReadLeases = mutableMapOf<String, Int>()
    private val maintenanceSessions = mutableSetOf<String>()
    private val historicalDatabases = mutableMapOf<String, MutableSet<SQLiteDatabase>>()
    private val historicalCancellationSignals = mutableMapOf<String, MutableSet<CancellationSignal>>()
    private val deletingSessions = mutableSetOf<String>()

    init {
        reconcileCatalog(catalog.writableDatabase)
        // A process death cannot leave an in-flight maintenance lease alive.
        // Keep its bounded cursor and counters, but make it resumable on the
        // next explicit maintenance request.
        catalog.writableDatabase.execSQL(
            "UPDATE session_maintenance_job SET state = 'pending' WHERE singleton = 1 AND state IN ('running', 'cancel_requested')",
        )
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
            val savedDevice = ensureSavedDevice(catalogDb, deviceId, advertisedName, null, at)
            val profile = boardProfile(catalogDb, savedDevice.savedDeviceId, at)
            val settings = settings(catalogDb)
            db.execSQL(
                """
                INSERT INTO session_metadata(
                  singleton, expected_riding_persistence_ms, expected_idle_persistence_ms
                ) VALUES (1, ?, ?)
                """.trimIndent(),
                arrayOf(settings.ridingPersistIntervalMs, settings.idlePersistIntervalMs),
            )
            db.execSQL(
                "INSERT INTO commit_marker(singleton, committed_at_ms) VALUES (1, ?)",
                arrayOf(started),
            )
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
    ): SavedDeviceRecord = ensureSavedDevice(catalog.writableDatabase, deviceId, advertisedName, null, at)

    @Synchronized
    override fun ensureSavedDevice(
        deviceId: DeviceId,
        advertisedName: String?,
        stableIdentity: String?,
        at: Instant,
    ): SavedDeviceRecord = ensureSavedDevice(
        catalog.writableDatabase,
        deviceId,
        advertisedName,
        stableIdentity,
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
        reconcileDirtyCatalog(record.jobId.value)
        val db = requireNotNull(sessionDb)
        db.beginTransaction()
        try {
            var previousTransitions = latestTransitionValues(db).toMutableMap()
            var eventSequence = android.database.DatabaseUtils.longForQuery(
                db,
                "SELECT COALESCE(MAX(sequence), 0) + 1 FROM session_event",
                null,
            )
            for (sample in samples) {
                val extensionJson = telemetryExtensionJson(sample.extension)
                db.execSQL(
                    """
                    INSERT OR IGNORE INTO sample(
                      sequence, observed_at_ms, slow_observed_at_ms, monotonic_elapsed_ms,
                      ride_state, package_state, package_mode, stop_condition,
                      setpoint_adjustment, wheelslip, darkride, charging, alert_reason,
                      firmware_fault, raw_pack_voltage_v, duty_cycle, controller_temp_c,
                      motor_temp_c, footpad_state, extension_json,
                      motor_current_a, battery_current_a, erpm, speed_kph,
                      pitch_deg, balance_pitch_deg, roll_deg, directional_motor_current_a,
                      filtered_motor_current_a, requested_balance_current_a, booster_current_a,
                      field_weakening_current_a, footpad_adc_1_v, footpad_adc_2_v, remote_input,
                      setpoint_deg, atr_setpoint_deg, brake_tilt_setpoint_deg, torque_tilt_setpoint_deg,
                      turn_tilt_setpoint_deg, remote_setpoint_deg,
                      atr_acceleration_difference_erpm_per_second, atr_speed_boost, distance_m,
                      odometer_m, amp_hours_drawn_ah, amp_hours_charged_ah, watt_hours_drawn_wh,
                      watt_hours_charged_wh, battery_level, charging_current_a, charging_voltage_v
                    ) VALUES (
                      ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                      ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                      ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                      ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
                    )
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
                        extensionJson,
                        sample.motorCurrentA,
                        sample.batteryCurrentA,
                        sample.erpm,
                        sample.speedKph,
                        sample.pitchDeg,
                        sample.balancePitchDeg,
                        sample.rollDeg,
                        sample.directionalMotorCurrentA,
                        sample.filteredMotorCurrentA,
                        sample.requestedBalanceCurrentA,
                        sample.boosterCurrentA,
                        sample.fieldWeakeningCurrentA,
                        sample.footpadAdc1V,
                        sample.footpadAdc2V,
                        sample.remoteInput,
                        sample.setpointDeg,
                        sample.atrSetpointDeg,
                        sample.brakeTiltSetpointDeg,
                        sample.torqueTiltSetpointDeg,
                        sample.turnTiltSetpointDeg,
                        sample.remoteSetpointDeg,
                        sample.atrAccelerationDifferenceErpmPerSecond,
                        sample.atrSpeedBoost,
                        sample.distanceM,
                        sample.odometerM,
                        sample.ampHoursDrawnAh,
                        sample.ampHoursChargedAh,
                        sample.wattHoursDrawnWh,
                        sample.wattHoursChargedWh,
                        sample.batteryLevel,
                        sample.chargingCurrentA,
                        sample.chargingVoltageV,
                    ),
                )
                val inserted = android.database.DatabaseUtils.longForQuery(
                    db,
                    "SELECT changes()",
                    null,
                ) == 1L
                if (!inserted) continue
                stableNumericValues(sample).forEach { (field, value) ->
                    if (value != null) {
                        db.execSQL(
                            "INSERT OR IGNORE INTO session_numeric_field(field_name, source, first_sequence) VALUES (?, 'stable', ?)",
                            arrayOf(field, sample.sequence),
                        )
                    }
                }
                sample.extension.keys.forEach { field ->
                    db.execSQL(
                        "INSERT OR IGNORE INTO session_numeric_field(field_name, source, first_sequence) VALUES (?, 'extension', ?)",
                        arrayOf(field, sample.sequence),
                    )
                }
                transitionValues(sample).forEach { (kind, value) ->
                    if (previousTransitions[kind] == value) return@forEach
                    db.execSQL(
                        """
                        INSERT INTO session_event(
                          sequence, source_sample_sequence, observed_at_ms, monotonic_elapsed_ms, kind, details_json
                        ) VALUES (?, ?, ?, ?, ?, ?)
                        """.trimIndent(),
                        arrayOf(
                            eventSequence,
                            sample.sequence,
                            sample.observedAt.toEpochMilli(),
                            sample.elapsedMs,
                            kind,
                            JSONObject().put("value", value).toString(),
                        ),
                    )
                    previousTransitions[kind] = value
                    eventSequence += 1
                }
            }
            // Android API 29 ships an SQLite build without the window-function
            // support used by the desktop marker query. Compute the same
            // ordered aggregates in Kotlin and keep the SQL update primitive.
            data class Aggregate(
                var previousElapsed: Long? = null,
                var previousSpeedKph: Double? = null,
                var ridingElapsed: Long = 0,
                var distanceM: Double = 0.0,
                var maximumSpeedKph: Double? = null,
                var maximumSpeedSequence: Long? = null,
                var maximumDutyCycle: Double? = null,
                var maximumDutySequence: Long? = null,
                var committedElapsed: Long = 0,
                var lastSpeedKph: Double? = null,
                var lastRideState: String? = null,
            )
            val aggregate = Aggregate()
            db.rawQuery(
                """
                SELECT sequence, monotonic_elapsed_ms, ride_state, speed_kph, duty_cycle
                FROM sample ORDER BY monotonic_elapsed_ms ASC, sequence ASC
                """.trimIndent(),
                null,
            ).use { cursor ->
                while (cursor.moveToNext()) {
                    val sequence = cursor.getLong(0)
                    val elapsed = cursor.getLong(1)
                    val rideState = cursor.getString(2)
                    val speedKph = if (cursor.isNull(3)) null else cursor.getDouble(3)
                    val dutyCycle = if (cursor.isNull(4)) null else cursor.getDouble(4)
                    aggregate.previousElapsed?.let { previousElapsed ->
                        val deltaMs = maxOf(0L, elapsed - previousElapsed)
                        if (rideState == "riding") aggregate.ridingElapsed += deltaMs
                        if (speedKph != null) {
                            aggregate.distanceM +=
                                ((kotlin.math.abs(aggregate.previousSpeedKph ?: speedKph) + kotlin.math.abs(speedKph)) / 2.0) *
                                    deltaMs / 3_600.0
                        }
                    }
                    val absoluteSpeed = speedKph?.let { kotlin.math.abs(it) }
                    if (absoluteSpeed != null &&
                        (aggregate.maximumSpeedKph == null || absoluteSpeed > aggregate.maximumSpeedKph!!)
                    ) {
                        aggregate.maximumSpeedKph = absoluteSpeed
                        aggregate.maximumSpeedSequence = sequence
                    }
                    val absoluteDuty = dutyCycle?.let { kotlin.math.abs(it) }
                    if (absoluteDuty != null &&
                        (aggregate.maximumDutyCycle == null || absoluteDuty > aggregate.maximumDutyCycle!!)
                    ) {
                        aggregate.maximumDutyCycle = absoluteDuty
                        aggregate.maximumDutySequence = sequence
                    }
                    aggregate.previousElapsed = elapsed
                    aggregate.previousSpeedKph = speedKph
                    aggregate.committedElapsed = elapsed
                    aggregate.lastSpeedKph = speedKph
                    aggregate.lastRideState = rideState
                }
            }
            val last = samples.maxOf { it.sequence }
            val finalEventSequence = DatabaseUtils.longForQuery(
                db,
                "SELECT COALESCE(MAX(sequence), 0) FROM session_event",
                null,
            )
            val storedSampleCount = DatabaseUtils.longForQuery(db, "SELECT COUNT(*) FROM sample", null)
            val eventCount = DatabaseUtils.longForQuery(db, "SELECT COUNT(*) FROM session_event", null)
            db.execSQL(
                """
                UPDATE commit_marker SET
                  committed_sample_sequence = ?, committed_event_sequence = ?,
                  recorded_sample_count = recorded_sample_count + ?, stored_sample_count = ?,
                  committed_event_count = ?, committed_elapsed_ms = ?, committed_at_ms = ?,
                  riding_elapsed_ms = ?, distance_m = ?, maximum_speed_kph = ?,
                  maximum_speed_source_sequence = ?, maximum_duty_cycle = ?,
                  maximum_duty_source_sequence = ?, last_speed_kph = ?, last_ride_state = ?
                WHERE singleton = 1
                """.trimIndent(),
                arrayOf(
                    maxOf(record.committedSequence, last), finalEventSequence, samples.size, storedSampleCount,
                    eventCount, aggregate.committedElapsed, samples.last().observedAt.toEpochMilli(),
                    aggregate.ridingElapsed, aggregate.distanceM, aggregate.maximumSpeedKph,
                    aggregate.maximumSpeedSequence, aggregate.maximumDutyCycle,
                    aggregate.maximumDutySequence, aggregate.lastSpeedKph, aggregate.lastRideState,
                ),
            )
            db.setTransactionSuccessful()
            current = record.copy(committedSequence = maxOf(record.committedSequence, last))
        } finally {
            db.endTransaction()
        }
        try {
            // The session commit is durable even when catalog convergence
            // fails. Mark the row dirty and let the marker-driven reconciler
            // repair it before the next append/read/publication.
            updateCatalogSummary(record, samples)
        } catch (error: Exception) {
            markCatalogDirty(record.jobId.value)
            throw error
        }
    }

    @Synchronized
    override fun appendLocations(jobId: JobId, observations: List<LocationObservation>) {
        val record = requireNotNull(current) { "No active session" }
        require(record.jobId == jobId) { "Wrong logging job" }
        if (observations.isEmpty()) return
        val db = requireNotNull(sessionDb)
        val ordered = observations.sortedWith(compareBy<LocationObservation> { it.elapsedMs }.thenBy { it.observedAt })
        db.beginTransaction()
        var lastSequence = 0L
        var acceptedCount = 0
        try {
            var sequence = DatabaseUtils.longForQuery(db, "SELECT COALESCE(MAX(location_sequence), 0) FROM location", null)
            ordered.forEach { observation ->
                require(observation.elapsedMs >= 0 && observation.latitude in -90.0..90.0 && observation.longitude in -180.0..180.0)
                require(observation.accuracyM == null || observation.accuracyM.isFinite() && observation.accuracyM >= 0.0)
                val duplicate = DatabaseUtils.longForQuery(
                    db,
                    "SELECT EXISTS(SELECT 1 FROM location WHERE monotonic_elapsed_ms = ? AND latitude = ? AND longitude = ?)",
                    arrayOf(observation.elapsedMs.toString(), observation.latitude.toString(), observation.longitude.toString()),
                ) == 1L
                if (duplicate) return@forEach
                sequence += 1
                db.execSQL(
                    "INSERT INTO location(location_sequence, observed_at_ms, monotonic_elapsed_ms, latitude, longitude, accuracy_m) VALUES (?, ?, ?, ?, ?, ?)",
                    arrayOf(sequence, observation.observedAt.toEpochMilli(), observation.elapsedMs, observation.latitude, observation.longitude, observation.accuracyM),
                )
                lastSequence = sequence
                acceptedCount += 1
            }
            if (lastSequence > 0) {
                val lastElapsed = DatabaseUtils.longForQuery(db, "SELECT monotonic_elapsed_ms FROM location WHERE location_sequence = ?", arrayOf(lastSequence.toString()))
                db.execSQL(
                    "UPDATE commit_marker SET committed_location_sequence = ?, committed_location_elapsed_ms = ?, recorded_location_count = recorded_location_count + ?, stored_location_count = (SELECT COUNT(*) FROM location), last_location_status = 'available' WHERE singleton = 1",
                    arrayOf(lastSequence, lastElapsed, acceptedCount),
                )
            }
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
        val marker = db.rawQuery(
            "SELECT committed_location_sequence, committed_location_elapsed_ms, recorded_location_count, stored_location_count, last_location_status FROM commit_marker WHERE singleton = 1",
            null,
        ).use { cursor ->
            check(cursor.moveToFirst())
            arrayOf(cursor.getLong(0), cursor.getLong(1), cursor.getLong(2), cursor.getLong(3), cursor.getString(4))
        }
        catalog.writableDatabase.execSQL(
            "UPDATE session_catalog SET committed_location_sequence = ?, committed_location_elapsed_ms = ?, location_count = ?, recorded_location_count = ?, stored_location_count = ?, location_status = ? WHERE job_id = ?",
            arrayOf(marker[0], marker[1], marker[2], marker[2], marker[3], marker[4], jobId.value),
        )
    }

    @Synchronized
    override fun close(jobId: JobId, at: Instant, reason: String): SessionRecord {
        val record = requireNotNull(current) { "No active session" }
        require(record.jobId == jobId) { "Wrong logging job" }
        reconcileDirtyCatalog(record.jobId.value)
        val enteredRiding = sessionDb?.let {
            android.database.DatabaseUtils.longForQuery(
                it,
                "SELECT EXISTS(SELECT 1 FROM sample WHERE ride_state = 'riding')",
                null,
            ) == 1L
        } ?: false
        val distance = catalog.writableDatabase.rawQuery(
            "SELECT COALESCE(distance_m, 0) FROM session_catalog WHERE job_id = ?",
            arrayOf(jobId.value),
        ).use { cursor ->
            if (cursor.moveToFirst()) cursor.getDouble(0) else 0.0
        }
        val retainSession = enteredRiding &&
            distance >= settings(catalog.writableDatabase).minimumSessionDistance
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
            catalogDb.setTransactionSuccessful()
        } finally {
            catalogDb.endTransaction()
        }
        current = null
        if (!retainSession) deleteSession(record.sessionId.value)
        return record.copy(active = false)
    }

    @Synchronized
    override fun active(): SessionRecord? = current

    @Synchronized
    override fun sessions(query: SessionListQuery): SessionPage {
        reconcileDirtyCatalog()
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
    override fun session(sessionId: String): SessionSummaryRecord {
        reconcileDirtyCatalog()
        return catalog.writableDatabase.rawQuery(
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
    }

    @Synchronized
    override fun sessionSnapshot(sessionId: String): SessionPublishedSnapshotRecord {
        reconcileDirtyCatalog()
        return catalog.writableDatabase.rawQuery(
            """
            SELECT history_revision, committed_sequence, committed_event_sequence,
                   committed_elapsed_ms, sample_count, stored_sample_count,
                   committed_location_sequence, committed_location_elapsed_ms,
                   location_count, recorded_location_count, stored_location_count, location_status
            FROM session_catalog WHERE session_id = ? AND pending_delete = 0
            """.trimIndent(),
            arrayOf(sessionId),
        ).use { cursor ->
            if (!cursor.moveToFirst()) throw SessionStoreException.SessionNotFound
            SessionPublishedSnapshotRecord(
                historyRevision = cursor.getLong(0),
                committedSampleSequence = cursor.getLong(1),
                committedEventSequence = cursor.getLong(2),
                committedElapsedMs = cursor.getLong(3),
                recordedSampleCount = cursor.getLong(4),
                storedSampleCount = cursor.getLong(5),
                committedLocationSequence = cursor.getLong(6),
                committedLocationElapsedMs = cursor.getLong(7),
                recordedLocationCount = cursor.getLong(9),
                storedLocationCount = cursor.getLong(10),
                locationStatus = cursor.getString(11),
            )
        }
    }

    override fun sessionHistoryConfiguration(sessionId: String): SessionHistoryConfiguration =
        withReadLease(sessionId) { db, cancellationSignal ->
            db.rawQuery(
                """
                SELECT expected_riding_persistence_ms, expected_idle_persistence_ms,
                       COALESCE(retained_resolution_seconds, 0)
                FROM session_metadata WHERE singleton = 1
                """.trimIndent(),
                null,
                cancellationSignal,
            ).use { cursor ->
                if (!cursor.moveToFirst()) throw SessionStoreException.SessionNotFound
                SessionHistoryConfiguration(cursor.getLong(0), cursor.getLong(1), cursor.getLong(2))
            }
        }

    override fun sessionPruneMetadata(sessionId: String): SessionPruneMetadataRecord =
        withReadLease(sessionId) { db, cancellationSignal ->
            db.rawQuery(
                """
                SELECT retained_resolution_seconds, prune_policy
                FROM session_metadata WHERE singleton = 1
                """.trimIndent(),
                null,
                cancellationSignal,
            ).use { cursor ->
                if (!cursor.moveToFirst()) throw SessionStoreException.SessionNotFound
                val snapshot = sessionSnapshot(sessionId)
                SessionPruneMetadataRecord(
                    if (cursor.isNull(0)) null else cursor.getLong(0),
                    snapshot.recordedSampleCount,
                    snapshot.storedSampleCount,
                    if (cursor.isNull(1)) null else cursor.getString(1),
                )
            }
        }

    @Synchronized
    override fun prunePreview(sessionId: String, targetResolutionSeconds: Long): SessionPrunePreviewRecord {
        val selection = pruneSelection(sessionId, targetResolutionSeconds)
        return SessionPrunePreviewRecord(
            recordedSampleCount = selection.recordedCount,
            estimatedStoredSampleCount = selection.keep.size.toLong(),
            estimatedReclaimableBytes = ((selection.rows.size - selection.keep.size).coerceAtLeast(0) * 256L),
            skipped = selection.keep.size == selection.rows.size,
            skipReason = if (selection.keep.size == selection.rows.size) "already_at_or_below_target" else null,
        )
    }

    override fun cancelHistoricalReads(sessionId: String) {
        val (signals, databases) = synchronized(leaseLock) {
            (historicalCancellationSignals[sessionId]?.toList() ?: emptyList()) to
                (historicalDatabases[sessionId]?.toList() ?: emptyList())
        }
        signals.forEach { signal -> runCatching { signal.cancel() } }
        databases.forEach { database -> runCatching { database.close() } }
    }

    @Synchronized
    override fun pruneSession(sessionId: String, targetResolutionSeconds: Long): SessionPrunePreviewRecord {
        val selection = pruneSelection(sessionId, targetResolutionSeconds)
        if (selection.keep.size == selection.rows.size) return SessionPrunePreviewRecord(
            selection.recordedCount, selection.keep.size.toLong(), 0, true, "already_at_or_below_target",
        )
        synchronized(leaseLock) {
            if ((activeReadLeases[sessionId] ?: 0) != 0 || !maintenanceSessions.add(sessionId)) {
                throw SessionStoreException.MaintenanceInProgress
            }
        }
        try {
        val fileName = catalog.writableDatabase.rawQuery(
            "SELECT file_name FROM session_catalog WHERE session_id = ? AND pending_delete = 0",
            arrayOf(sessionId),
        ).use { cursor ->
            if (!cursor.moveToFirst()) throw SessionStoreException.SessionNotFound
            cursor.getString(0)
        }
        val db = openSession(sessionId)
        try {
            db.beginTransaction()
            try {
                db.execSQL("PRAGMA temp_store = FILE")
                db.execSQL("CREATE TEMP TABLE prune_keep(sequence INTEGER PRIMARY KEY)")
                selection.keep.forEach { sequence ->
                    db.execSQL("INSERT INTO prune_keep(sequence) VALUES (?)", arrayOf(sequence))
                }
                db.execSQL("DELETE FROM sample WHERE sequence NOT IN (SELECT sequence FROM prune_keep)")
                db.execSQL(
                    "DELETE FROM location WHERE NOT EXISTS (SELECT 1 FROM sample WHERE ABS(sample.monotonic_elapsed_ms - location.monotonic_elapsed_ms) <= ?)",
                    arrayOf(targetResolutionSeconds * 1_000),
                )
                db.execSQL(
                    """
                    UPDATE session_metadata SET pruned_at_ms = ?, retained_resolution_seconds = ?,
                      prune_policy = 'm4-source-v1'
                    WHERE singleton = 1
                    """.trimIndent(),
                    arrayOf(System.currentTimeMillis(), targetResolutionSeconds),
                )
                db.execSQL(
                    """
                    UPDATE commit_marker SET stored_sample_count = (SELECT COUNT(*) FROM sample),
                      stored_location_count = (SELECT COUNT(*) FROM location),
                      history_revision = history_revision + 1, committed_at_ms = ?
                    WHERE singleton = 1
                    """.trimIndent(),
                    arrayOf(System.currentTimeMillis()),
                )
                db.setTransactionSuccessful()
            } finally {
                db.endTransaction()
                runCatching { db.execSQL("DROP TABLE IF EXISTS prune_keep") }
            }
            // Logical pruning is durable before these bounded reclamation
            // steps. A cancelled/interrupted checkpoint can be resumed later.
            executePragma(db, "PRAGMA wal_checkpoint(PASSIVE)")
            executePragma(db, "PRAGMA incremental_vacuum(256)")
        } finally {
            db.close()
        }
        val jobId = catalog.writableDatabase.rawQuery(
            "SELECT job_id FROM session_catalog WHERE session_id = ?",
            arrayOf(sessionId),
        ).use { cursor -> if (cursor.moveToFirst()) cursor.getString(0) else throw SessionStoreException.SessionNotFound }
        reconcileSessionCatalog(sessionId, jobId, fileName)
        return SessionPrunePreviewRecord(
            selection.recordedCount, selection.keep.size.toLong(),
            ((selection.rows.size - selection.keep.size).coerceAtLeast(0) * 256L), false, null,
        )
        } finally {
            synchronized(leaseLock) { maintenanceSessions.remove(sessionId) }
        }
    }

    @Synchronized
    override fun maintenanceJob(): SessionMaintenanceJobRecord? =
        catalog.writableDatabase.rawQuery(
            """
            SELECT operation_id, kind, scope, state, session_id, target_resolution_seconds,
                   upper_session_id, last_session_id, completed_sessions, skipped_sessions,
                   retained_samples, reclaimable_bytes, error
            FROM session_maintenance_job WHERE singleton = 1
            """.trimIndent(),
            null,
        ).use { cursor ->
            if (!cursor.moveToFirst()) return@use null
            maintenanceJobFromCursor(cursor)
        }

    @Synchronized
    override fun saveMaintenanceJob(job: SessionMaintenanceJobRecord) {
        catalog.writableDatabase.execSQL(
            """
            INSERT OR REPLACE INTO session_maintenance_job(
              singleton, operation_id, kind, scope, session_id, state,
              target_resolution_seconds, upper_session_id, last_session_id,
              completed_sessions, skipped_sessions, retained_samples,
              reclaimable_bytes, error, updated_at_ms
            ) VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """.trimIndent(),
            arrayOf(
                job.operationId, job.kind, job.scope, job.sessionId, job.state,
                job.targetResolutionSeconds, job.upperSessionId, job.lastSessionId,
                job.completedSessions, job.skippedSessions, job.retainedSamples,
                job.reclaimableBytes, job.error, System.currentTimeMillis(),
            ),
        )
    }

    @Synchronized
    override fun cancelMaintenanceJob(operationId: String): SessionMaintenanceJobRecord? {
        val job = maintenanceJob() ?: return null
        if (job.operationId != operationId || job.state in setOf("completed", "cancelled", "failed")) return job
        val updated = job.copy(state = "cancel_requested")
        saveMaintenanceJob(updated)
        cancelHistoricalReads(job.sessionId ?: "")
        return updated
    }

    private fun maintenanceJobFromCursor(cursor: android.database.Cursor): SessionMaintenanceJobRecord =
        SessionMaintenanceJobRecord(
            operationId = cursor.getString(0),
            kind = cursor.getString(1),
            scope = cursor.getString(2),
            state = cursor.getString(3),
            sessionId = cursor.optionalString(4),
            targetResolutionSeconds = cursor.getLong(5),
            upperSessionId = cursor.optionalString(6),
            lastSessionId = cursor.optionalString(7),
            completedSessions = cursor.getLong(8),
            skippedSessions = cursor.getLong(9),
            retainedSamples = cursor.getLong(10),
            reclaimableBytes = cursor.getLong(11),
            error = cursor.optionalString(12),
        )

    private data class PruneSelection(
        val rows: List<SessionSeriesPoint>,
        val keep: List<Long>,
        val recordedCount: Long,
    )

    private fun pruneSelection(sessionId: String, targetResolutionSeconds: Long): PruneSelection {
        if (targetResolutionSeconds !in 1..300) throw SessionStoreException.InvalidRange
        if (current?.sessionId?.value == sessionId) throw SessionStoreException.SessionActive
        val existingResolution = withReadLease(sessionId) { db, cancellationSignal ->
            db.rawQuery(
                "SELECT retained_resolution_seconds FROM session_metadata WHERE singleton = 1",
                null,
                cancellationSignal,
            ).use { cursor ->
                if (!cursor.moveToFirst() || cursor.isNull(0)) null else cursor.getLong(0)
            }
        }
        if (existingResolution != null && existingResolution > 0 &&
            (targetResolutionSeconds <= existingResolution || targetResolutionSeconds % existingResolution != 0L)
        ) {
            throw SessionStoreException.InvalidRange
        }
        val rows = withReadLease(sessionId) { db, cancellationSignal ->
            db.rawQuery(
                "SELECT sequence, monotonic_elapsed_ms, ride_state, speed_kph, raw_pack_voltage_v FROM sample ORDER BY monotonic_elapsed_ms, sequence",
                null,
                cancellationSignal,
            ).use { cursor ->
                buildList {
                    while (cursor.moveToNext()) add(
                        SessionSeriesPoint(
                            elapsedMs = cursor.getLong(1),
                            values = mapOf(
                                "speedKph" to cursor.optionalDouble(3),
                                "packVoltageV" to cursor.optionalDouble(4),
                            ),
                            sequence = cursor.getLong(0),
                            rideState = cursor.getString(2),
                        ),
                    )
                }
            }
        }
        val protected = withReadLease(sessionId) { db, cancellationSignal ->
            buildSet {
                db.rawQuery("SELECT first_sequence FROM session_numeric_field", null, cancellationSignal).use { cursor ->
                    while (cursor.moveToNext()) add(cursor.getLong(0))
                }
                db.rawQuery("SELECT source_sample_sequence FROM session_event WHERE source_sample_sequence IS NOT NULL", null, cancellationSignal).use { cursor ->
                    while (cursor.moveToNext()) add(cursor.getLong(0))
                }
            }
        }
        val keep = SessionHistoryReducer.selectPruneRows(
            rows,
            listOf("speedKph", "packVoltageV"),
            targetResolutionSeconds,
            protectedSequences = protected,
            eventSourceSequences = emptySet(),
        )
        return PruneSelection(rows, keep, rows.size.toLong())
    }

    override fun querySeries(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        fields: List<String>,
        maxPoints: Int,
    ): List<SessionSeriesPoint> {
        return querySeries(sessionId, startElapsedMs, endElapsedMs, fields, maxPoints, null)
    }

    override fun querySeries(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        fields: List<String>,
        maxPoints: Int,
        publishedSampleSequence: Long?,
    ): List<SessionSeriesPoint> {
        if (startElapsedMs > endElapsedMs || maxPoints !in 1..2_000) {
            throw SessionStoreException.InvalidRange
        }
        return readSeries(
            sessionId, startElapsedMs, endElapsedMs, fields, maxPoints, false,
            publishedSampleSequence,
        )
    }

    override fun querySeriesSource(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        fields: List<String>,
        maxRows: Int,
        publishedSampleSequence: Long?,
    ): List<SessionSeriesPoint> {
        if (startElapsedMs > endElapsedMs || maxRows !in 1..200_000) {
            throw SessionStoreException.InvalidRange
        }
        return readSeries(
            sessionId, startElapsedMs, endElapsedMs, fields, maxRows, true,
            publishedSampleSequence,
        )
    }

    override fun reduceSeries(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        fields: List<String>,
        buckets: Int,
        gapPolicy: SessionGapPolicyRecord,
        expectedPersistenceMs: Long,
        expectedRidingPersistenceMs: Long?,
        expectedIdlePersistenceMs: Long?,
        retainedResolutionSeconds: Long,
        publishedSampleSequence: Long?,
    ): SessionSeriesReductionRecord {
        if (startElapsedMs > endElapsedMs) throw SessionStoreException.InvalidRange
        val projection = seriesProjection(fields)
        val sequencePredicate = if (publishedSampleSequence == null) "" else " AND sequence <= ?"
        val arguments = buildList {
            add(startElapsedMs.toString())
            add(endElapsedMs.toString())
            publishedSampleSequence?.let { add(it.toString()) }
            add("200000")
        }.toTypedArray()
        val sql = """
            SELECT sequence, monotonic_elapsed_ms, ride_state, ${projection.selected} FROM sample
            WHERE monotonic_elapsed_ms BETWEEN ? AND ?$sequencePredicate
            ORDER BY monotonic_elapsed_ms, sequence LIMIT ?
        """.trimIndent()
        return withReadLease(sessionId) { db, cancellationSignal ->
            val accumulator = SessionHistoryReducer.SeriesAccumulator(
                projection.fields,
                SessionElapsedRange(startElapsedMs, endElapsedMs),
                buckets,
                gapPolicy,
                expectedPersistenceMs,
                expectedRidingPersistenceMs,
                expectedIdlePersistenceMs,
                retainedResolutionSeconds,
            )
            val started = System.nanoTime()
            val rowValues = MutableList<Double?>(projection.fields.size) { null }
            var rowCount = 0
            db.rawQuery(sql, arguments, cancellationSignal).use { cursor ->
                while (cursor.moveToNext()) {
                    cursor.populateSeriesValues(projection, rowValues)
                    accumulator.append(
                        elapsedMs = cursor.getLong(1),
                        sequence = cursor.getLong(0),
                        rideState = cursor.getString(2),
                        values = rowValues,
                    )
                    rowCount += 1
                }
            }
            val reducedColumns = accumulator.finish()
            val elapsedNanos = System.nanoTime() - started
            SessionSeriesReductionRecord(
                columns = reducedColumns,
                sourceRows = rowCount,
                sourceMs = elapsedNanos / 1_000_000.0,
                reducerMs = 0.0,
                elapsedMs = elapsedNanos / 1_000_000.0,
            )
        }
    }

    override fun querySampleWindow(
        sessionId: String,
        centerElapsedMs: Long,
        beforeMs: Long,
        afterMs: Long,
        fields: List<String>,
    ): List<SessionSeriesPoint> {
        return querySampleWindow(sessionId, centerElapsedMs, beforeMs, afterMs, fields, null)
    }

    override fun querySampleWindow(
        sessionId: String,
        centerElapsedMs: Long,
        beforeMs: Long,
        afterMs: Long,
        fields: List<String>,
        publishedSampleSequence: Long?,
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
            publishedSampleSequence,
        )
    }

    override fun sessionEvents(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
    ): List<SessionEventRecord> {
        return sessionEvents(sessionId, startElapsedMs, endElapsedMs, null)
    }

    override fun sessionEvents(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        publishedEventSequence: Long?,
    ): List<SessionEventRecord> {
        if (startElapsedMs > endElapsedMs) throw SessionStoreException.InvalidRange
        return withReadLease(sessionId) { db, cancellationSignal ->
            val sequencePredicate = if (publishedEventSequence == null) "" else " AND sequence <= ?"
            val arguments = buildList {
                add(startElapsedMs.toString())
                add(endElapsedMs.toString())
                publishedEventSequence?.let { add(it.toString()) }
            }.toTypedArray()
            db.rawQuery(
                """
                SELECT sequence, source_sample_sequence, monotonic_elapsed_ms, kind, details_json FROM (
                  SELECT sequence, source_sample_sequence, monotonic_elapsed_ms, kind, details_json
                  FROM session_event WHERE monotonic_elapsed_ms BETWEEN ? AND ?
                    $sequencePredicate
                    AND kind IN (
                      'packageState', 'packageMode', 'footpadState', 'stopCondition',
                      'setpointAdjustment', 'wheelslip', 'darkride', 'charging',
                      'alertReason', 'firmwareFault'
                    )
                  ORDER BY sequence DESC LIMIT 501
                ) ORDER BY monotonic_elapsed_ms, sequence
                """.trimIndent(),
                arguments,
                cancellationSignal,
            ).use { cursor ->
                buildList {
                    while (cursor.moveToNext()) {
                        add(
                            SessionEventRecord(
                                cursor.getLong(0).toString(),
                                cursor.getLong(2),
                                cursor.getString(3),
                                cursor.getString(4),
                                if (cursor.isNull(1)) null else cursor.getLong(1),
                            ),
                        )
                    }
                }
            }
        }
    }

    override fun queryLocations(
        sessionId: String,
        startElapsedMs: Long,
        endElapsedMs: Long,
        limit: Int,
        cursor: String?,
        publishedLocationSequence: Long?,
    ): SessionLocationPageRecord {
        if (startElapsedMs > endElapsedMs || limit !in 1..2_000) throw SessionStoreException.InvalidRange
        val snapshot = sessionSnapshot(sessionId)
        if (publishedLocationSequence != null && publishedLocationSequence != snapshot.committedLocationSequence) {
            throw SessionStoreException.InvalidRange
        }
        val afterSequence = cursor?.let {
            try {
                val value = JSONObject(String(Base64.decode(it, Base64.DEFAULT), Charsets.UTF_8))
                require(value.getString("sessionId") == sessionId)
                require(value.getLong("startElapsedMs") == startElapsedMs)
                require(value.getLong("endElapsedMs") == endElapsedMs)
                require(value.getLong("historyRevision") == snapshot.historyRevision)
                require(value.getLong("locationSequence") >= 0)
                value.getLong("locationSequence")
            } catch (_: Exception) {
                throw SessionStoreException.InvalidCursor
            }
        } ?: 0L
        return withReadLease(sessionId) { db, cancellationSignal ->
            val predicates = mutableListOf("monotonic_elapsed_ms BETWEEN ? AND ?")
            val args = mutableListOf(startElapsedMs.toString(), endElapsedMs.toString())
            publishedLocationSequence?.let {
                predicates += "location_sequence <= ?"
                args += it.toString()
            }
            if (afterSequence > 0) {
                predicates += "location_sequence > ?"
                args += afterSequence.toString()
            }
            args += (limit + 1).toString()
            val rows = db.rawQuery(
                """
                SELECT location_sequence, monotonic_elapsed_ms, latitude, longitude, accuracy_m, observed_at_ms
                FROM location WHERE ${predicates.joinToString(" AND ")}
                ORDER BY monotonic_elapsed_ms, location_sequence LIMIT ?
                """.trimIndent(),
                args.toTypedArray(),
                cancellationSignal,
            ).use { cursorValue ->
                buildList {
                    while (cursorValue.moveToNext()) add(
                        SessionLocationPointRecord(
                            locationSequence = cursorValue.getLong(0),
                            elapsedMs = cursorValue.getLong(1),
                            latitude = cursorValue.getDouble(2),
                            longitude = cursorValue.getDouble(3),
                            accuracyM = if (cursorValue.isNull(4)) null else cursorValue.getDouble(4),
                            observedAt = Instant.ofEpochMilli(cursorValue.getLong(5)),
                        ),
                    )
                }
            }
            val page = rows.take(limit)
            val gaps = page.zipWithNext().mapNotNull { (left, right) ->
                if (right.elapsedMs - left.elapsedMs > 10_000) {
                    SessionLocationGapRecord(left.elapsedMs + 1, right.elapsedMs - 1, "noFix")
                } else null
            }
            val nextCursor = if (rows.size > limit && page.isNotEmpty()) {
                Base64.encodeToString(
                    JSONObject()
                        .put("sessionId", sessionId)
                        .put("startElapsedMs", startElapsedMs)
                        .put("endElapsedMs", endElapsedMs)
                        .put("historyRevision", snapshot.historyRevision)
                        .put("locationSequence", page.last().locationSequence)
                        .toString().toByteArray(Charsets.UTF_8),
                    Base64.NO_WRAP,
                )
            } else null
            SessionLocationPageRecord(page, gaps, nextCursor, snapshot)
        }
    }

    override fun sessionExtensionFields(sessionId: String): List<String> =
        withReadLease(sessionId) { db, cancellationSignal ->
            db.rawQuery(
                """
                SELECT field_name FROM session_numeric_field
                WHERE source = 'extension'
                ORDER BY field_name
                LIMIT 33
                """.trimIndent(),
                null,
                cancellationSignal,
            ).use { cursor ->
                val fields = linkedSetOf<String>()
                while (cursor.moveToNext()) {
                    fields += cursor.getString(0)
                }
                check(fields.size <= 32) { "Stored telemetry extensions exceed their bound" }
                fields.sorted()
            }
        }

    @Synchronized
    override fun savedDevices(): List<SavedDeviceRecord> {
        val db = catalog.writableDatabase
        return db.rawQuery(
            """
            SELECT d.saved_device_id, d.device_history_id, d.revision,
                   d.custom_display_name, d.advertised_name, d.opaque_platform_id,
                   d.last_seen_at_ms, d.last_connected_at_ms, COALESCE(p.json, '{}'),
                   COUNT(s.session_id), COALESCE(SUM(s.riding_elapsed_ms), 0),
                   SUM(s.distance_m), MAX(s.started_at_ms)
            FROM saved_device d
            LEFT JOIN board_profile p ON p.saved_device_id = d.saved_device_id
            LEFT JOIN session_catalog s
              ON s.device_history_id = d.device_history_id AND s.pending_delete = 0
            GROUP BY d.saved_device_id, d.device_history_id, d.revision,
                     d.custom_display_name, d.advertised_name, d.opaque_platform_id,
                     d.last_seen_at_ms, d.last_connected_at_ms, p.json, d.created_at_ms
            ORDER BY COALESCE(d.last_connected_at_ms, d.created_at_ms) DESC
            """.trimIndent(),
            null,
        ).use { cursor ->
            buildList {
                while (cursor.moveToNext()) {
                    val valuesJson = cursor.getString(8)
                    val customName = cursor.optionalString(3)
                    val advertisedName = cursor.optionalString(4)
                    val identifier = cursor.optionalString(5)
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
                            summary = BoardHistorySummaryRecord(
                                sessionCount = cursor.getInt(9),
                                totalRidingElapsedMs = cursor.getLong(10),
                                totalDistanceM = cursor.optionalDouble(11),
                                mostRecentSessionAt = cursor.optionalInstant(12),
                            ),
                        ),
                    )
                }
            }
        }
    }

    @Synchronized
    override fun deleteSavedDevice(savedDeviceId: String) {
        if (current != null) throw ProductStoreException.SavedDeviceActive
        val db = catalog.writableDatabase
        val historyId = db.rawQuery(
            "SELECT device_history_id FROM saved_device WHERE saved_device_id = ?",
            arrayOf(savedDeviceId),
        ).use { cursor ->
            if (!cursor.moveToFirst()) throw ProductStoreException.SavedDeviceNotFound
            cursor.getString(0)
        }
        val files = db.rawQuery(
            "SELECT file_name FROM session_catalog WHERE device_history_id = ?",
            arrayOf(historyId),
        ).use { cursor ->
            buildList {
                while (cursor.moveToNext()) add(cursor.getString(0))
            }
        }
        db.beginTransaction()
        try {
            db.delete("board_profile", "saved_device_id = ?", arrayOf(savedDeviceId))
            db.delete("controller_route_hint", "saved_device_id = ?", arrayOf(savedDeviceId))
            db.delete("session_catalog", "device_history_id = ?", arrayOf(historyId))
            val deleted = db.delete("saved_device", "saved_device_id = ?", arrayOf(savedDeviceId))
            if (deleted != 1) throw ProductStoreException.SavedDeviceNotFound
            db.setTransactionSuccessful()
        } finally {
            db.endTransaction()
        }
        files.forEach { fileName ->
            val file = File(root, fileName)
            file.delete()
            File(file.path + "-wal").delete()
            File(file.path + "-shm").delete()
        }
    }

    @Synchronized
    override fun ensureImportedSavedDevice(
        deviceHistoryId: String,
        displayName: String?,
        profileSnapshotJson: String,
        at: Instant,
    ): SavedDeviceRecord = ensureImportedSavedDevice(
        catalog.writableDatabase,
        deviceHistoryId,
        displayName,
        profileSnapshotJson,
        at,
    )

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
            "SELECT file_name, state, pending_delete FROM session_catalog WHERE session_id = ?",
            arrayOf(sessionId),
        ).use { cursor ->
            if (cursor.moveToFirst()) Triple(cursor.getString(0), cursor.getString(1), cursor.getInt(2) != 0) else null
        } ?: return
        if (sessionRow.second == "active") throw SessionStoreException.SessionActive
        if (sessionRow.third) return
        val fileName = sessionRow.first
        db.execSQL(
            "UPDATE session_catalog SET pending_delete = 1 WHERE session_id = ?",
            arrayOf(sessionId),
        )
        val hasReaders = synchronized(leaseLock) { (activeReadLeases[sessionId] ?: 0) > 0 }
        if (hasReaders) return
        finalizePendingDelete(sessionId, fileName)
    }

    @Synchronized
    override fun cancelPendingDelete(operationId: String): Boolean {
        if (deletingSessions.contains(operationId)) return false
        val db = catalog.writableDatabase
        val row = db.rawQuery(
            "SELECT file_name, pending_delete FROM session_catalog WHERE session_id = ?",
            arrayOf(operationId),
        ).use { cursor ->
            if (cursor.moveToFirst()) cursor.getString(0) to (cursor.getInt(1) != 0) else null
        } ?: return false
        if (!row.second || !File(root, row.first).exists()) return false
        db.execSQL(
            "UPDATE session_catalog SET pending_delete = 0 WHERE session_id = ? AND pending_delete = 1",
            arrayOf(operationId),
        )
        return DatabaseUtils.longForQuery(db, "SELECT changes()", null) == 1L
    }

    private fun finalizePendingDelete(sessionId: String, fileName: String) {
        if (synchronized(leaseLock) { (activeReadLeases[sessionId] ?: 0) > 0 }) return
        if (!deletingSessions.add(sessionId)) return
        try {
            val sessionFile = File(root, fileName)
            if (sessionFile.exists() && !SQLiteDatabase.deleteDatabase(sessionFile)) {
                error("Unable to remove durable session files")
            }
            val db = catalog.writableDatabase
            db.beginTransaction()
            try {
                db.execSQL("DELETE FROM session_catalog WHERE session_id = ?", arrayOf(sessionId))
                db.setTransactionSuccessful()
            } finally {
                db.endTransaction()
            }
        } finally {
            deletingSessions.remove(sessionId)
        }
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
                    writeCsv(session, output, settings(db).includeGpsLocationWhenExporting)
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

    @Synchronized
    override fun importSessionArchive(data: ByteArray): String {
        if (data.size > 128 * 1024 * 1024) throw SessionStoreException.InvalidArchive
        if (current != null) throw SessionStoreException.SessionActive

        val importId = UUID.randomUUID().toString()
        val sessionId = importId
        val jobId = UUID.randomUUID().toString()
        val fileName = "session-$sessionId.sqlite"
        val temporary = File(root, "import-$importId.floatview.tmp")
        val destination = File(root, fileName)
        fun removeSQLiteSidecars(file: File) {
            File(file.path + "-wal").delete()
            File(file.path + "-shm").delete()
        }
        try {
            temporary.outputStream().use { it.write(data) }
            val manifest = SQLiteDatabase.openDatabase(
                temporary.path,
                null,
                SQLiteDatabase.OPEN_READWRITE,
            ).use { archive ->
                val formatVersion = archive.rawQuery(
                    "SELECT value FROM float_view_archive_metadata WHERE key = 'format_version'",
                    null,
                ).use { cursor -> if (cursor.moveToFirst()) cursor.getString(0) else null }
                val manifestText = archive.rawQuery(
                    "SELECT value FROM float_view_archive_metadata WHERE key = 'session'",
                    null,
                ).use { cursor -> if (cursor.moveToFirst()) cursor.getString(0) else null }
                val healthy = archive.rawQuery("PRAGMA integrity_check", null).use { cursor ->
                    cursor.moveToFirst() && cursor.getString(0) == "ok"
                }
                val hasMarker = DatabaseUtils.longForQuery(
                    archive,
                    "SELECT EXISTS(SELECT 1 FROM commit_marker WHERE singleton = 1)",
                    null,
                ) == 1L
                val sampleCount = DatabaseUtils.longForQuery(archive, "SELECT COUNT(*) FROM sample", null)
                if (formatVersion != "1" || manifestText.isNullOrEmpty() || !healthy || !hasMarker || sampleCount == 0L) {
                    throw SessionStoreException.InvalidArchive
                }
                JSONObject(manifestText)
            }
            removeSQLiteSidecars(temporary)

            val deviceHistoryId = (manifest.opt("deviceHistoryId") as? String)
                ?.takeIf(::isValidDeviceHistoryId)
                ?: throw SessionStoreException.InvalidArchive
            val deviceSnapshot = manifest.optJSONObject("deviceSnapshot")?.toString() ?: "{}"
            val profileSnapshot = manifest.optJSONObject("boardProfileSnapshot")?.toString() ?: "{}"
            val displayName = manifest.optJSONObject("deviceSnapshot")?.optString("displayName")
                ?.takeIf(String::isNotEmpty)
            val startedAtMs = manifest.optLong("startedAtMs", System.currentTimeMillis())
            val endedAtMs = if (manifest.has("endedAtMs") && !manifest.isNull("endedAtMs")) {
                manifest.optLong("endedAtMs")
            } else {
                null
            }
            val requestedIdleMs = manifest.optLong("requestedIdleMs", 1_000L).coerceAtLeast(1L)
            val effectiveIdleMs = manifest.optLong("effectiveIdleMs", requestedIdleMs).coerceAtLeast(1L)
            val requestedRidingMs = manifest.optLong("requestedRidingMs", 1_000L).coerceAtLeast(1L)
            val effectiveRidingMs = manifest.optLong("effectiveRidingMs", requestedRidingMs).coerceAtLeast(1L)
            val closeReason = if (manifest.has("closeReason") && !manifest.isNull("closeReason")) {
                manifest.optString("closeReason")
            } else {
                null
            }

            if (!temporary.renameTo(destination)) throw SessionStoreException.InvalidArchive
            val catalogDb = catalog.writableDatabase
            catalogDb.beginTransaction()
            try {
                ensureImportedSavedDevice(
                    catalogDb,
                    deviceHistoryId,
                    displayName,
                    profileSnapshot,
                    Instant.ofEpochMilli(startedAtMs),
                )
                catalogDb.execSQL(
                    """
                    INSERT INTO session_catalog(
                      session_id, job_id, device_history_id, device_snapshot_json,
                      profile_snapshot_json, profile_revision, file_name, started_at_ms,
                      ended_at_ms, state, integrity, requested_idle_ms, effective_idle_ms,
                      requested_riding_ms, effective_riding_ms, close_reason
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'completed', 'complete', ?, ?, ?, ?, ?)
                    """.trimIndent(),
                    arrayOf(
                        sessionId,
                        jobId,
                        deviceHistoryId,
                        deviceSnapshot,
                        profileSnapshot,
                        manifest.optLong("boardProfileRevision", 0L),
                        fileName,
                        startedAtMs,
                        endedAtMs,
                        requestedIdleMs,
                        effectiveIdleMs,
                        requestedRidingMs,
                        effectiveRidingMs,
                        closeReason,
                    ),
                )
                catalogDb.setTransactionSuccessful()
            } finally {
                catalogDb.endTransaction()
            }
            reconcileSessionCatalog(sessionId, jobId, fileName)
            return sessionId
        } catch (error: SessionStoreException) {
            removeSQLiteSidecars(temporary)
            temporary.delete()
            SQLiteDatabase.deleteDatabase(destination)
            throw error
        } catch (_: Exception) {
            removeSQLiteSidecars(temporary)
            temporary.delete()
            SQLiteDatabase.deleteDatabase(destination)
            throw SessionStoreException.InvalidArchive
        }
    }

    @Synchronized
    fun recoverInterrupted(): Int {
        val db = catalog.writableDatabase
        data class ActiveSession(val sessionId: String, val fileName: String, val distance: Double)
        val activeSessions = db.rawQuery(
            "SELECT session_id, file_name, COALESCE(distance_m, 0) FROM session_catalog WHERE state = 'active'",
            null,
        ).use { cursor ->
            buildList {
                while (cursor.moveToNext()) {
                    add(ActiveSession(cursor.getString(0), cursor.getString(1), cursor.getDouble(2)))
                }
            }
        }
        val minimumDistance = settings(db).minimumSessionDistance
        var recovered = 0
        activeSessions.forEach { active ->
            val enteredRiding = runCatching {
                SQLiteDatabase.openDatabase(
                    File(root, active.fileName).path,
                    null,
                    SQLiteDatabase.OPEN_READONLY,
                ).use { session ->
                    android.database.DatabaseUtils.longForQuery(
                        session,
                        "SELECT EXISTS(SELECT 1 FROM sample WHERE ride_state = 'riding')",
                        null,
                    ) == 1L
                }
            }.getOrDefault(false)
            db.execSQL(
                "UPDATE session_catalog SET state = 'interrupted', integrity = 'recovered', close_reason = 'process_interruption' WHERE session_id = ? AND state = 'active'",
                arrayOf(active.sessionId),
            )
            if (enteredRiding && active.distance >= minimumDistance) {
                recovered += 1
            } else {
                deleteSession(active.sessionId)
            }
        }
        return recovered
    }

    private fun settings(db: SQLiteDatabase): SettingsRecord {
        val existing = db.rawQuery(
            "SELECT json FROM settings WHERE singleton = 1",
            null,
        ).use { cursor ->
            if (cursor.moveToFirst()) {
                try {
                    val values = JSONObject(cursor.getString(0))
                    if (!values.has("minimumSessionDistance")) {
                        values.put("minimumSessionDistance", SettingsRecord().minimumSessionDistance)
                    }
                    if (!values.has("historyGapThresholdSeconds")) {
                        values.put("historyGapThresholdSeconds", JSONObject.NULL)
                    }
                    if (!values.has("locationRecordingEnabled")) {
                        values.put("locationRecordingEnabled", false)
                    }
                    if (!values.has("includeGpsLocationWhenExporting")) {
                        values.put("includeGpsLocationWhenExporting", false)
                    }
                    validatedSettings(values, values.optInt("revision", 0))
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
        .put("minimumSessionDistance", value.minimumSessionDistance)
        .put("themePreference", value.themePreference.wireValue)
        .put("unitPreference", value.unitPreference.wireValue)
        .put("historyGapThresholdSeconds", value.historyGapThresholdSeconds ?: JSONObject.NULL)
        .put("locationRecordingEnabled", value.locationRecordingEnabled)
        .put("includeGpsLocationWhenExporting", value.includeGpsLocationWhenExporting)

    private fun validatedSettings(values: JSONObject, revision: Int): SettingsRecord {
        fun integer(name: String, range: LongRange): Long {
            val raw = values.opt(name)
            if (raw !is Number || raw.toDouble() % 1.0 != 0.0) {
                throw ProductStoreException.InvalidSettingsPatch
            }
            return raw.toLong().coerceIn(range)
        }
        fun double(name: String, range: ClosedFloatingPointRange<Double>): Double {
            val raw = values.opt(name) as? Number
                ?: throw ProductStoreException.InvalidSettingsPatch
            val value = raw.toDouble()
            if (!value.isFinite()) throw ProductStoreException.InvalidSettingsPatch
            return value.coerceIn(range)
        }
        fun themePreference(name: String): ThemePreference =
            (values.opt(name) as? String)
                ?.let(ThemePreference::fromWire)
                ?: throw ProductStoreException.InvalidSettingsPatch
        fun unitPreference(name: String): UnitPreference =
            (values.opt(name) as? String)
                ?.let(UnitPreference::fromWire)
                ?: throw ProductStoreException.InvalidSettingsPatch
        fun historyGapThresholdSeconds(name: String): Double? {
            val raw = values.opt(name)
            if (raw == null || raw == JSONObject.NULL) return null
            val value = (raw as? Number)?.toDouble()
                ?: throw ProductStoreException.InvalidSettingsPatch
            if (!value.isFinite()) throw ProductStoreException.InvalidSettingsPatch
            return value.coerceIn(0.1..3_600.0)
        }
        fun boolean(name: String): Boolean {
            val raw = values.opt(name)
            if (raw !is Boolean) throw ProductStoreException.InvalidSettingsPatch
            return raw
        }
        return SettingsRecord(
            revision = revision,
            idleProbeIntervalMs = integer("idleProbeIntervalMs", 100L..10_000L),
            idlePersistIntervalMs = integer("idlePersistIntervalMs", 100L..60_000L),
            ridingPersistIntervalMs = integer("ridingPersistIntervalMs", 50L..10_000L),
            postRideTailMs = integer("postRideTailMs", 0L..300_000L),
            reconnectDurationMs = integer("reconnectDurationMs", 0L..3_600_000L),
            minimumSessionDistance = double("minimumSessionDistance", 0.0..1_000_000.0),
            themePreference = themePreference("themePreference"),
            unitPreference = unitPreference("unitPreference"),
            historyGapThresholdSeconds = historyGapThresholdSeconds("historyGapThresholdSeconds"),
            locationRecordingEnabled = boolean("locationRecordingEnabled"),
            includeGpsLocationWhenExporting = boolean("includeGpsLocationWhenExporting"),
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

    private fun writeCsv(db: SQLiteDatabase, output: File, includeGpsLocation: Boolean) {
        BufferedWriter(OutputStreamWriter(FileOutputStream(output), Charsets.UTF_8), 64 * 1024).use { writer ->
            writer.appendLine(
                "sequence,observed_at_ms,slow_observed_at_ms,elapsed_ms,ride_state," +
                    "package_state,package_mode,footpad_state,stop_condition," +
                    "setpoint_adjustment,wheelslip,darkride,charging,alert_reason," +
                    "firmware_fault,pack_voltage_v,duty_cycle,controller_temp_c," +
                    "motor_temp_c,motor_current_a,battery_current_a,erpm,speed_kph," +
                    "pitchDeg,balancePitchDeg,rollDeg,directionalMotorCurrentA," +
                    "filteredMotorCurrentA,requestedBalanceCurrentA,boosterCurrentA," +
                    "fieldWeakeningCurrentA,footpadAdc1V,footpadAdc2V,remoteInput," +
                    "setpointDeg,atrSetpointDeg,brakeTiltSetpointDeg,torqueTiltSetpointDeg," +
                    "turnTiltSetpointDeg,remoteSetpointDeg," +
                    "atrAccelerationDifferenceErpmPerSecond,atrSpeedBoost,distanceM,odometerM," +
                    "ampHoursDrawnAh,ampHoursChargedAh,wattHoursDrawnWh,wattHoursChargedWh," +
                    "batteryLevel,chargingCurrentA,chargingVoltageV,extension_json" +
                        if (includeGpsLocation) ",gpsAccuracy,gpsLongitude,gpsLatitude" else "",
            )
            db.rawQuery(
                """
                SELECT sequence, observed_at_ms, slow_observed_at_ms, monotonic_elapsed_ms,
                       ride_state, package_state, package_mode, footpad_state,
                       stop_condition, setpoint_adjustment, wheelslip, darkride, charging,
                       alert_reason, firmware_fault, raw_pack_voltage_v, duty_cycle,
                       controller_temp_c, motor_temp_c, motor_current_a, battery_current_a,
                       erpm, speed_kph, pitch_deg, balance_pitch_deg, roll_deg,
                       directional_motor_current_a, filtered_motor_current_a,
                       requested_balance_current_a, booster_current_a, field_weakening_current_a,
                       footpad_adc_1_v, footpad_adc_2_v, remote_input, setpoint_deg,
                       atr_setpoint_deg, brake_tilt_setpoint_deg, torque_tilt_setpoint_deg,
                       turn_tilt_setpoint_deg, remote_setpoint_deg,
                       atr_acceleration_difference_erpm_per_second, atr_speed_boost, distance_m,
                       odometer_m, amp_hours_drawn_ah, amp_hours_charged_ah, watt_hours_drawn_wh,
                       watt_hours_charged_wh, battery_level, charging_current_a, charging_voltage_v,
                       extension_json
                FROM sample ORDER BY sequence
                """.trimIndent(),
                null,
            ).use { cursor ->
                while (cursor.moveToNext()) {
                    val columns = (0 until cursor.columnCount).map { index ->
                            if (cursor.isNull(index)) "" else if (
                                index in setOf(4, 5, 6, 7, 8, 9, 13, 14, 51)
                            ) {
                                csv(cursor.getString(index))
                            } else {
                                cursor.getString(index)
                            }
                        }.toMutableList()
                    if (includeGpsLocation) {
                        columns += nearestLocationCsvColumns(db, cursor.getLong(3))
                    }
                    writer.appendLine(columns.joinToString(","))
                }
            }
        }
    }

    private fun nearestLocationCsvColumns(db: SQLiteDatabase, elapsedMs: Long): List<String> {
        val tolerance = CSV_LOCATION_TOLERANCE_MS
        return db.rawQuery(
            """
            SELECT accuracy_m, longitude, latitude
            FROM location
            WHERE monotonic_elapsed_ms BETWEEN ? AND ?
            ORDER BY ABS(monotonic_elapsed_ms - ?) ASC, location_sequence ASC
            LIMIT 1
            """.trimIndent(),
            arrayOf((elapsedMs - tolerance).toString(), (elapsedMs + tolerance).toString(), elapsedMs.toString()),
        ).use { cursor ->
            if (!cursor.moveToFirst()) return listOf("", "", "")
            listOf(
                cursor.optionalString(0)?.toDoubleOrNull()?.toString() ?: "",
                cursor.getDouble(1).toString(),
                cursor.getDouble(2).toString(),
            )
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

    private fun markCatalogDirty(jobId: String) {
        catalog.writableDatabase.execSQL(
            "UPDATE session_catalog SET integrity = 'dirty' WHERE job_id = ?",
            arrayOf(jobId),
        )
    }

    /** Rebuild catalog boundaries and aggregates from the authoritative marker. */
    private fun reconcileSessionCatalog(sessionId: String, jobId: String, fileName: String) {
        val file = File(root, fileName)
        if (!file.exists()) {
            markCatalogDirty(jobId)
            return
        }
        val session = SQLiteDatabase.openDatabase(file.path, null, SQLiteDatabase.OPEN_READONLY)
        try {
            val healthy = session.rawQuery("PRAGMA integrity_check", null).use { cursor ->
                cursor.moveToFirst() && cursor.getString(0) == "ok"
            }
            if (!healthy) {
                markCatalogDirty(jobId)
                return
            }
            data class Marker(
                val sampleSequence: Long,
                val eventSequence: Long,
                val recordedCount: Long,
                val storedCount: Long,
                val eventCount: Long,
                val elapsed: Long,
                val revision: Long,
                val locationSequence: Long,
                val locationElapsed: Long,
                val locationCount: Long,
                val storedLocationCount: Long,
                val locationStatus: String,
            )
            val marker = session.rawQuery(
                """
                SELECT committed_sample_sequence, committed_event_sequence,
                       recorded_sample_count, stored_sample_count, committed_event_count,
                       committed_elapsed_ms, history_revision,
                       committed_location_sequence, committed_location_elapsed_ms,
                       recorded_location_count, stored_location_count, last_location_status
                FROM commit_marker WHERE singleton = 1
                """.trimIndent(),
                null,
            ).use { cursor ->
                if (!cursor.moveToFirst()) error("Missing session commit marker")
                Marker(
                    cursor.getLong(0), cursor.getLong(1), cursor.getLong(2),
                    cursor.getLong(3), cursor.getLong(4), cursor.getLong(5), cursor.getLong(6),
                    cursor.getLong(7), cursor.getLong(8), cursor.getLong(9), cursor.getLong(10), cursor.getString(11),
                )
            }
            var previousElapsed: Long? = null
            var previousSpeed: Double? = null
            var ridingElapsed = 0L
            var distance = 0.0
            var maxSpeed: Double? = null
            var maxSpeedSequence: Long? = null
            var maxDuty: Double? = null
            var maxDutySequence: Long? = null
            var lastElapsed: Long? = null
            var lastSpeed: Double? = null
            session.rawQuery(
                """
                SELECT sequence, monotonic_elapsed_ms, ride_state, speed_kph, duty_cycle
                FROM sample ORDER BY monotonic_elapsed_ms ASC, sequence ASC
                """.trimIndent(),
                null,
            ).use { cursor ->
                while (cursor.moveToNext()) {
                    val sequence = cursor.getLong(0)
                    val elapsed = cursor.getLong(1)
                    val riding = cursor.getString(2) == "riding"
                    val speed = if (cursor.isNull(3)) null else kotlin.math.abs(cursor.getDouble(3)) / 3.6
                    val duty = if (cursor.isNull(4)) null else kotlin.math.abs(cursor.getDouble(4))
                    previousElapsed?.let { prior ->
                        val delta = maxOf(0L, elapsed - prior)
                        if (riding) ridingElapsed += delta
                        if (speed != null) distance += ((previousSpeed ?: speed) + speed) * 0.5 * delta / 1_000.0
                    }
                    if (speed != null && (maxSpeed == null || speed > maxSpeed!!)) {
                        maxSpeed = speed
                        maxSpeedSequence = sequence
                    }
                    if (duty != null && (maxDuty == null || duty > maxDuty!!)) {
                        maxDuty = duty
                        maxDutySequence = sequence
                    }
                    previousElapsed = elapsed
                    previousSpeed = speed
                    lastElapsed = elapsed
                    lastSpeed = speed
                }
            }
            val storedCount = DatabaseUtils.longForQuery(session, "SELECT COUNT(*) FROM sample", null)
            val actualEventCount = DatabaseUtils.longForQuery(session, "SELECT COUNT(*) FROM session_event", null)
            val actualEventSequence = DatabaseUtils.longForQuery(
                session, "SELECT COALESCE(MAX(sequence), 0) FROM session_event", null,
            )
            catalog.writableDatabase.execSQL(
                """
                UPDATE session_catalog SET integrity = CASE WHEN integrity = 'dirty' THEN 'complete' ELSE integrity END,
                  committed_sequence = ?, committed_event_sequence = ?,
                  sample_count = ?, stored_sample_count = ?, event_count = ?,
                  committed_elapsed_ms = ?, history_revision = ?,
                  riding_elapsed_ms = ?, total_elapsed_ms = ?, distance_m = ?,
                  max_speed_mps = ?, max_duty_cycle = ?,
                  max_speed_source_sequence = ?, max_duty_source_sequence = ?,
                  last_sample_elapsed_ms = ?, last_speed_mps = ?,
                  committed_location_sequence = ?, committed_location_elapsed_ms = ?,
                  location_count = ?, recorded_location_count = ?, stored_location_count = ?, location_status = ?
                WHERE session_id = ? AND job_id = ?
                """.trimIndent(),
                arrayOf(
                    marker.sampleSequence,
                    maxOf(marker.eventSequence, actualEventSequence),
                    marker.recordedCount,
                    maxOf(marker.storedCount, storedCount),
                    maxOf(marker.eventCount, actualEventCount),
                    maxOf(marker.elapsed, lastElapsed ?: 0L),
                    marker.revision,
                    ridingElapsed,
                    lastElapsed ?: 0L,
                    distance,
                    maxSpeed,
                    maxDuty,
                    maxSpeedSequence,
                    maxDutySequence,
                    lastElapsed,
                    lastSpeed,
                    marker.locationSequence,
                    marker.locationElapsed,
                    marker.locationCount,
                    marker.locationCount,
                    marker.storedLocationCount,
                    marker.locationStatus,
                    sessionId,
                    jobId,
                ),
            )
        } finally {
            session.close()
        }
    }

    private fun reconcileCatalog(db: SQLiteDatabase) {
        db.rawQuery(
            "SELECT session_id, job_id, file_name FROM session_catalog WHERE pending_delete = 0",
            null,
        ).use { cursor ->
            while (cursor.moveToNext()) {
                val sessionId = cursor.getString(0)
                val jobId = cursor.getString(1)
                try {
                    reconcileSessionCatalog(sessionId, jobId, cursor.getString(2))
                } catch (_: Exception) {
                    markCatalogDirty(jobId)
                }
            }
        }
    }

    private fun reconcileDirtyCatalog(jobId: String) {
        catalog.writableDatabase.rawQuery(
            """
            SELECT session_id, file_name FROM session_catalog
            WHERE job_id = ? AND pending_delete = 0 AND integrity = 'dirty'
            """.trimIndent(),
            arrayOf(jobId),
        ).use { cursor ->
            while (cursor.moveToNext()) {
                reconcileSessionCatalog(cursor.getString(0), jobId, cursor.getString(1))
            }
        }
    }

    private fun reconcileDirtyCatalog() {
        catalog.writableDatabase.rawQuery(
            """
            SELECT session_id, job_id, file_name FROM session_catalog
            WHERE pending_delete = 0 AND integrity = 'dirty'
            """.trimIndent(),
            null,
        ).use { cursor ->
            while (cursor.moveToNext()) {
                reconcileSessionCatalog(cursor.getString(0), cursor.getString(1), cursor.getString(2))
            }
        }
    }

    private fun updateCatalogSummary(record: SessionRecord, samples: List<TelemetrySample>) {
        val sessionDatabase = requireNotNull(sessionDb)
        val marker = sessionDatabase.rawQuery(
            """
            SELECT committed_sample_sequence, committed_event_sequence,
                   recorded_sample_count, stored_sample_count, committed_event_count,
                   committed_elapsed_ms, riding_elapsed_ms, distance_m,
                   maximum_speed_kph, maximum_duty_cycle,
                   maximum_speed_source_sequence, maximum_duty_source_sequence,
                   last_speed_kph, committed_location_sequence,
                   committed_location_elapsed_ms, recorded_location_count, stored_location_count, last_location_status
            FROM commit_marker WHERE singleton = 1
            """.trimIndent(),
            null,
        ).use { cursor ->
            check(cursor.moveToFirst())
            arrayOf<Any?>(
                cursor.getLong(0), cursor.getLong(1), cursor.getLong(2), cursor.getLong(3),
                cursor.getLong(4), cursor.getLong(5), cursor.getLong(6),
                if (cursor.isNull(7)) null else cursor.getDouble(7),
                if (cursor.isNull(8)) null else cursor.getDouble(8),
                if (cursor.isNull(9)) null else cursor.getDouble(9),
                if (cursor.isNull(10)) null else cursor.getLong(10),
                if (cursor.isNull(11)) null else cursor.getLong(11),
                if (cursor.isNull(12)) null else cursor.getDouble(12),
                cursor.getLong(13), cursor.getLong(14), cursor.getLong(15), cursor.getLong(16), cursor.getString(17),
            )
        }
        val db = catalog.writableDatabase
        db.execSQL(
            """
            UPDATE session_catalog SET committed_sequence = ?, committed_event_sequence = ?,
              committed_elapsed_ms = ?, stored_sample_count = ?, sample_count = ?,
              event_count = ?, riding_elapsed_ms = ?, total_elapsed_ms = ?, distance_m = ?,
              max_speed_mps = ?, max_duty_cycle = ?, max_speed_source_sequence = ?,
              max_duty_source_sequence = ?, last_sample_elapsed_ms = ?,
              last_speed_mps = ?, committed_location_sequence = ?,
              committed_location_elapsed_ms = ?, location_count = ?, recorded_location_count = ?, stored_location_count = ?, location_status = ? WHERE job_id = ?
            """.trimIndent(),
            arrayOf(
                marker[0], marker[1], marker[5], marker[3], marker[2], marker[4], marker[6], marker[5], marker[7],
                (marker[8] as Double?)?.let { kotlin.math.abs(it) / 3.6 },
                (marker[9] as Double?)?.let { kotlin.math.abs(it) },
                marker[10], marker[11], marker[5],
                (marker[12] as Double?)?.let { kotlin.math.abs(it) / 3.6 },
                marker[13], marker[14], marker[15], marker[15], marker[16], marker[17],
                record.jobId.value,
            ),
        )
    }

    private fun openSession(sessionId: String): SQLiteDatabase {
        catalog.writableDatabase.rawQuery(
            "SELECT job_id FROM session_catalog WHERE session_id = ? AND pending_delete = 0",
            arrayOf(sessionId),
        ).use { cursor ->
            if (cursor.moveToFirst()) reconcileDirtyCatalog(cursor.getString(0))
        }
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

    /**
     * Resolve a historical file under a short monitor, then scan it on a
     * dedicated read-only/query-only WAL connection. The lease prevents a
     * pending deletion from removing the file while the connection is alive,
     * while the semaphore bounds native historical work to two readers.
     */
    private fun <T> withReadLease(sessionId: String, body: (SQLiteDatabase, CancellationSignal) -> T): T {
        historicalReadSlots.acquire()
        var file: File? = null
        var leased = false
        try {
            synchronized(this) {
                reconcileDirtyCatalog()
                val fileName = catalog.writableDatabase.rawQuery(
                    "SELECT file_name FROM session_catalog WHERE session_id = ? AND pending_delete = 0",
                    arrayOf(sessionId),
                ).use { cursor ->
                    if (!cursor.moveToFirst()) throw SessionStoreException.SessionNotFound
                    cursor.getString(0)
                }
                file = File(root, fileName)
                if (!file!!.exists()) throw SessionStoreException.SessionNotFound
                synchronized(leaseLock) {
                    if (maintenanceSessions.contains(sessionId)) {
                        throw SessionStoreException.MaintenanceInProgress
                    }
                    activeReadLeases[sessionId] = (activeReadLeases[sessionId] ?: 0) + 1
                }
                leased = true
            }
            val database = SQLiteDatabase.openDatabase(file!!.path, null, SQLiteDatabase.OPEN_READONLY)
            val cancellationSignal = CancellationSignal()
            synchronized(leaseLock) {
                historicalDatabases.getOrPut(sessionId) { linkedSetOf() }.add(database)
                historicalCancellationSignals.getOrPut(sessionId) { linkedSetOf() }.add(cancellationSignal)
            }
            try {
                // OPEN_READONLY connections reject execSQL transaction calls,
                // but SQLite still permits a deferred read transaction when it
                // is prepared as a query. This pins one WAL snapshot for the
                // complete bounded read without blocking the writer.
                database.rawQuery("BEGIN DEFERRED TRANSACTION", null).use { }
                return body(database, cancellationSignal)
            } finally {
                try { database.rawQuery("ROLLBACK", null).use { } } catch (_: Exception) { }
                synchronized(leaseLock) {
                    historicalDatabases[sessionId]?.remove(database)
                    if (historicalDatabases[sessionId].isNullOrEmpty()) historicalDatabases.remove(sessionId)
                    historicalCancellationSignals[sessionId]?.remove(cancellationSignal)
                    if (historicalCancellationSignals[sessionId].isNullOrEmpty()) {
                        historicalCancellationSignals.remove(sessionId)
                    }
                }
                database.close()
            }
        } finally {
            var lastLease = false
            synchronized(leaseLock) {
                if (leased) {
                    val count = activeReadLeases[sessionId] ?: 0
                    if (count <= 1) {
                        activeReadLeases.remove(sessionId)
                        lastLease = true
                    } else {
                        activeReadLeases[sessionId] = count - 1
                    }
                }
            }
            if (lastLease) {
                synchronized(this) {
                    catalog.writableDatabase.rawQuery(
                        "SELECT file_name FROM session_catalog WHERE session_id = ? AND pending_delete = 1",
                        arrayOf(sessionId),
                    ).use { cursor ->
                        if (cursor.moveToFirst()) finalizePendingDelete(sessionId, cursor.getString(0))
                    }
                }
            }
            historicalReadSlots.release()
        }
    }

    private fun readSeries(
        sessionId: String,
        start: Long,
        end: Long,
        fields: List<String>,
        maxPoints: Int,
        lossless: Boolean,
        maximumSequence: Long?,
    ): List<SessionSeriesPoint> {
        val projection = seriesProjection(fields)
        val sql: String
        val arguments: Array<String>
        val sequencePredicate = if (maximumSequence == null) "" else " AND sequence <= ?"
        arguments = buildList {
            add(start.toString())
            add(end.toString())
            maximumSequence?.let { add(it.toString()) }
            add(maxPoints.toString())
        }.toTypedArray()
        sql = """
            SELECT sequence, monotonic_elapsed_ms, ride_state, ${projection.selected} FROM sample
            WHERE monotonic_elapsed_ms BETWEEN ? AND ?$sequencePredicate
            ORDER BY monotonic_elapsed_ms, sequence LIMIT ?
        """.trimIndent()
        return withReadLease(sessionId) { db, cancellationSignal ->
            db.rawQuery(sql, arguments, cancellationSignal).use { cursor ->
                buildList {
                    while (cursor.moveToNext()) {
                        add(
                            SessionSeriesPoint(
                                cursor.getLong(1),
                                cursor.seriesValues(projection),
                                cursor.getLong(0),
                                cursor.getString(2),
                            ),
                        )
                    }
                }
            }
        }
    }

    private data class SeriesProjection(
        val fields: List<String>,
        val selected: String,
        val extensionColumn: Int?,
    )

    /**
     * Android API 29 does not ship SQLite's JSON1 extension. Keep extension
     * values in their bounded JSON column and decode only the requested numeric
     * keys after the indexed sample scan, while stable fields remain direct
     * nullable columns.
     */
    private fun seriesProjection(fields: List<String>): SeriesProjection {
        val uniqueFields = fields.distinct()
        if (uniqueFields.isEmpty() || uniqueFields.size > 16) throw SessionStoreException.InvalidRange
        uniqueFields.forEach { field ->
            if (SERIES_COLUMNS[field] == null &&
                (field.isEmpty() || field.toByteArray(Charsets.UTF_8).size > 64)
            ) {
                throw SessionStoreException.UnsupportedField(field)
            }
        }
        val hasExtensions = uniqueFields.any { SERIES_COLUMNS[it] == null }
        val selected = buildString {
            append(uniqueFields.joinToString(", ") { SERIES_COLUMNS[it] ?: "NULL" })
            if (hasExtensions) append(", extension_json")
        }
        return SeriesProjection(
            uniqueFields,
            selected,
            if (hasExtensions) 3 + uniqueFields.size else null,
        )
    }

    private fun android.database.Cursor.seriesDouble(
        index: Int,
        field: String,
        extensionColumn: Int?,
    ): Double? {
        if (SERIES_COLUMNS[field] != null) return optionalDouble(index)
        val payloadIndex = extensionColumn ?: return null
        if (isNull(payloadIndex)) return null
        return try {
            val payload = JSONObject(getString(payloadIndex))
            if (!payload.has(field) || payload.isNull(field)) null
            else payload.get(field).let { value ->
                (value as? Number)?.toDouble()?.takeIf(Double::isFinite)
            }
        } catch (_: Exception) {
            null
        }
    }

    private fun android.database.Cursor.seriesValues(projection: SeriesProjection): Map<String, Double?> {
        val values = MutableList<Double?>(projection.fields.size) { null }
        populateSeriesValues(projection, values)
        return projection.fields.zip(values).toMap()
    }

    private fun android.database.Cursor.populateSeriesValues(
        projection: SeriesProjection,
        values: MutableList<Double?>,
    ) {
        val extension = projection.extensionColumn?.takeIf { !isNull(it) }?.let { index ->
            try {
                JSONObject(getString(index))
            } catch (_: Exception) {
                null
            }
        }
        projection.fields.forEachIndexed { index, field ->
            values[index] = if (SERIES_COLUMNS[field] != null) {
                optionalDouble(index + 3)
            } else {
                val value = extension?.opt(field)
                (value as? Number)?.toDouble()?.takeIf(Double::isFinite)
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
        stableIdentity: String?,
        at: Instant,
    ): SavedDeviceRecord {
        val normalizedIdentity = stableIdentity?.takeIf(::isValidDeviceHistoryId)
        val stableMatch = normalizedIdentity?.let { identity ->
            db.rawQuery(
                "SELECT saved_device_id FROM saved_device WHERE device_history_id = ? LIMIT 1",
                arrayOf(identity),
            ).use { cursor -> if (cursor.moveToFirst()) cursor.getString(0) else null }
        }
        val localMatch = db.rawQuery(
            "SELECT saved_device_id FROM saved_device WHERE opaque_platform_id = ? LIMIT 1",
            arrayOf(deviceId.value),
        ).use { cursor -> if (cursor.moveToFirst()) cursor.getString(0) else null }
        val existingId = stableMatch ?: localMatch
        if (existingId != null) {
            db.execSQL(
                """
                UPDATE saved_device
                SET opaque_platform_id = ?, last_connected_at_ms = ?,
                    advertised_name = COALESCE(?, advertised_name)
                WHERE saved_device_id = ?
                """.trimIndent(),
                arrayOf(deviceId.value, at.toEpochMilli(), advertisedName, existingId),
            )
            boardProfile(db, existingId, at)
            return savedDevices().first { it.savedDeviceId == existingId }
        }
        val savedDeviceId = UUID.randomUUID().toString()
        val deviceHistoryId = normalizedIdentity ?: UUID.randomUUID().toString()
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
        return savedDevices().first { it.savedDeviceId == savedDeviceId }
    }

    private fun ensureImportedSavedDevice(
        db: SQLiteDatabase,
        deviceHistoryId: String,
        displayName: String?,
        profileSnapshotJson: String,
        at: Instant,
    ): SavedDeviceRecord {
        if (!isValidDeviceHistoryId(deviceHistoryId) || profileSnapshotJson.toByteArray().size > 16_384) {
            throw SessionStoreException.InvalidArchive
        }
        runCatching { JSONObject(profileSnapshotJson) }.getOrElse { throw SessionStoreException.InvalidArchive }
        val existingId = db.rawQuery(
            "SELECT saved_device_id FROM saved_device WHERE device_history_id = ? LIMIT 1",
            arrayOf(deviceHistoryId),
        ).use { cursor -> if (cursor.moveToFirst()) cursor.getString(0) else null }
        if (existingId != null) {
            boardProfile(db, existingId, at)
            return savedDevices().first { it.savedDeviceId == existingId }
        }
        val normalizedName = displayName?.trim()?.takeIf(String::isNotEmpty)
        if ((normalizedName?.toByteArray()?.size ?: 0) > 128) throw SessionStoreException.InvalidArchive
        val savedDeviceId = UUID.randomUUID().toString()
        db.execSQL(
            """
            INSERT INTO saved_device(
              saved_device_id, device_history_id, opaque_platform_id, advertised_name,
              custom_display_name, revision, created_at_ms, last_seen_at_ms, last_connected_at_ms
            ) VALUES (?, ?, NULL, ?, NULL, 0, ?, NULL, NULL)
            """.trimIndent(),
            arrayOf(savedDeviceId, deviceHistoryId, normalizedName, at.toEpochMilli()),
        )
        db.execSQL(
            "INSERT INTO board_profile(saved_device_id, revision, json, updated_at_ms) VALUES (?, 0, ?, ?)",
            arrayOf(savedDeviceId, profileSnapshotJson, at.toEpochMilli()),
        )
        return savedDevices().first { it.savedDeviceId == savedDeviceId }
    }

    private fun storeControllerRoutingHint(
        db: SQLiteDatabase,
        hint: ControllerRoutingHint,
        at: Instant,
    ) {
        db.execSQL(
            """
            INSERT OR REPLACE INTO controller_route_hint(
              saved_device_id, can_id, controller_uuid, updated_at_ms
            ) VALUES (?, ?, ?, ?)
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
            VALUES (?, 0, '{"preferredBmsCanId":null}', ?)
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
        bluetoothIdentifier: String?,
    ): String = customDisplayName?.takeIf(String::isNotBlank)
        ?: advertisedName?.takeIf(String::isNotBlank)
        ?: bluetoothIdentifier
        ?: "Board"

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
        if (!integerInRange("preferredBmsCanId", 0..253)) return false
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

    private fun stableNumericValues(sample: TelemetrySample): Map<String, Double?> = mapOf(
        "packVoltageV" to sample.packVoltageV, "dutyCycle" to sample.dutyCycle,
        "controllerTempC" to sample.controllerTempC, "motorTempC" to sample.motorTempC,
        "motorCurrentA" to sample.motorCurrentA, "batteryCurrentA" to sample.batteryCurrentA,
        "erpm" to sample.erpm, "speedKph" to sample.speedKph, "pitchDeg" to sample.pitchDeg,
        "balancePitchDeg" to sample.balancePitchDeg, "rollDeg" to sample.rollDeg,
        "directionalMotorCurrentA" to sample.directionalMotorCurrentA,
        "filteredMotorCurrentA" to sample.filteredMotorCurrentA,
        "requestedBalanceCurrentA" to sample.requestedBalanceCurrentA,
        "boosterCurrentA" to sample.boosterCurrentA, "fieldWeakeningCurrentA" to sample.fieldWeakeningCurrentA,
        "footpadAdc1V" to sample.footpadAdc1V, "footpadAdc2V" to sample.footpadAdc2V,
        "remoteInput" to sample.remoteInput, "setpointDeg" to sample.setpointDeg,
        "atrSetpointDeg" to sample.atrSetpointDeg, "brakeTiltSetpointDeg" to sample.brakeTiltSetpointDeg,
        "torqueTiltSetpointDeg" to sample.torqueTiltSetpointDeg, "turnTiltSetpointDeg" to sample.turnTiltSetpointDeg,
        "remoteSetpointDeg" to sample.remoteSetpointDeg,
        "atrAccelerationDifferenceErpmPerSecond" to sample.atrAccelerationDifferenceErpmPerSecond,
        "atrSpeedBoost" to sample.atrSpeedBoost, "distanceM" to sample.distanceM,
        "odometerM" to sample.odometerM, "ampHoursDrawnAh" to sample.ampHoursDrawnAh,
        "ampHoursChargedAh" to sample.ampHoursChargedAh, "wattHoursDrawnWh" to sample.wattHoursDrawnWh,
        "wattHoursChargedWh" to sample.wattHoursChargedWh, "batteryLevel" to sample.batteryLevel,
        "chargingCurrentA" to sample.chargingCurrentA, "chargingVoltageV" to sample.chargingVoltageV,
    )

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
        fun isValidDeviceHistoryId(value: String): Boolean {
            val bytes = value.toByteArray(Charsets.UTF_8)
            return bytes.isNotEmpty() && bytes.size <= 128 && bytes.all { byte ->
                val unsigned = byte.toInt() and 0xff
                unsigned in 48..57 || unsigned in 65..90 || unsigned in 97..122 ||
                    unsigned == 45 || unsigned == 46 || unsigned == 95
            }
        }
        val SETTING_KEYS = setOf(
            "idleProbeIntervalMs",
            "idlePersistIntervalMs",
            "ridingPersistIntervalMs",
            "postRideTailMs",
            "reconnectDurationMs",
            "minimumSessionDistance",
            "themePreference",
            "unitPreference",
            "historyGapThresholdSeconds",
            "locationRecordingEnabled",
            "includeGpsLocationWhenExporting",
        )
        const val MAX_PROFILE_BYTES = 65_536
        const val CSV_LOCATION_TOLERANCE_MS = 10_000L
        val SERIES_COLUMNS = mapOf(
            "packVoltageV" to "raw_pack_voltage_v",
            "dutyCycle" to "duty_cycle",
            "controllerTempC" to "controller_temp_c",
            "motorTempC" to "motor_temp_c",
            "motorCurrentA" to "motor_current_a",
            "batteryCurrentA" to "battery_current_a",
            "erpm" to "erpm",
            "speedKph" to "speed_kph",
            "pitchDeg" to "pitch_deg",
            "balancePitchDeg" to "balance_pitch_deg",
            "rollDeg" to "roll_deg",
            "directionalMotorCurrentA" to "directional_motor_current_a",
            "filteredMotorCurrentA" to "filtered_motor_current_a",
            "requestedBalanceCurrentA" to "requested_balance_current_a",
            "boosterCurrentA" to "booster_current_a",
            "fieldWeakeningCurrentA" to "field_weakening_current_a",
            "footpadAdc1V" to "footpad_adc_1_v",
            "footpadAdc2V" to "footpad_adc_2_v",
            "remoteInput" to "remote_input",
            "setpointDeg" to "setpoint_deg",
            "atrSetpointDeg" to "atr_setpoint_deg",
            "brakeTiltSetpointDeg" to "brake_tilt_setpoint_deg",
            "torqueTiltSetpointDeg" to "torque_tilt_setpoint_deg",
            "turnTiltSetpointDeg" to "turn_tilt_setpoint_deg",
            "remoteSetpointDeg" to "remote_setpoint_deg",
            "atrAccelerationDifferenceErpmPerSecond" to
                "atr_acceleration_difference_erpm_per_second",
            "atrSpeedBoost" to "atr_speed_boost",
            "distanceM" to "distance_m",
            "odometerM" to "odometer_m",
            "ampHoursDrawnAh" to "amp_hours_drawn_ah",
            "ampHoursChargedAh" to "amp_hours_charged_ah",
            "wattHoursDrawnWh" to "watt_hours_drawn_wh",
            "wattHoursChargedWh" to "watt_hours_charged_wh",
            "batteryLevel" to "battery_level",
            "chargingCurrentA" to "charging_current_a",
            "chargingVoltageV" to "charging_voltage_v",
        )
        val SESSION_SORT_COLUMNS = mapOf(
            "startedAt" to "started_at_ms",
            "totalElapsedMs" to "total_elapsed_ms",
            "ridingElapsedMs" to "riding_elapsed_ms",
            "distanceM" to "COALESCE(distance_m, -1)",
        )
    }
}
