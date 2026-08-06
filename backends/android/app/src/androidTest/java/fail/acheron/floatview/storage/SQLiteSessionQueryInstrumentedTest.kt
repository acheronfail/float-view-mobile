package fail.acheron.floatview.storage

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.os.OperationCanceledException
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import fail.acheron.floatview.backend.DeviceId
import fail.acheron.floatview.backend.LocationObservation
import fail.acheron.floatview.backend.PackageState
import fail.acheron.floatview.backend.RideState
import fail.acheron.floatview.backend.ProductStoreException
import fail.acheron.floatview.backend.SessionListQuery
import fail.acheron.floatview.backend.SessionMaintenanceJobRecord
import fail.acheron.floatview.backend.SessionReadCancellation
import fail.acheron.floatview.backend.SessionStoreException
import fail.acheron.floatview.backend.SetpointAdjustment
import fail.acheron.floatview.backend.TelemetrySample
import fail.acheron.floatview.backend.ThemePreference
import java.io.File
import java.time.Instant
import java.util.concurrent.CountDownLatch
import java.util.concurrent.ExecutionException
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import kotlin.math.abs
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class SQLiteSessionQueryInstrumentedTest {
    private lateinit var context: Context
    private lateinit var root: File

    @Before
    fun setUp() {
        context = ApplicationProvider.getApplicationContext()
        root = File(context.filesDir, "float-view")
        root.deleteRecursively()
    }

    @After
    fun tearDown() {
        root.deleteRecursively()
    }

    @Test
    fun catalogAndBoundedQueriesReadCanonicalSamples() {
        val store = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        val startedAt = Instant.ofEpochMilli(1_700_000_000_000)
        val record = store.create(DeviceId("query-fixture"), startedAt)
        store.append(
            record.jobId,
            (0 until 100).map { index ->
                TelemetrySample(
                    sequence = (index + 1).toLong(),
                    observedAt = startedAt.plusMillis(index * 1_000L),
                    elapsedMs = index * 1_000L,
                    rideState = if (index == 0) RideState.IDLE else RideState.RIDING,
                    packVoltageV = 60 - index / 10.0,
                    dutyCycle = index / 100.0,
                    controllerTempC = null,
                    motorTempC = null,
                    footpadState = null,
                    packageState = PackageState.entries[index % 4],
                    speedKph = 18.0,
                    pitchDeg = (index % 4).toDouble(),
                    extension = mapOf("future.energy" to index + 0.25),
                )
            },
        )
        store.close(record.jobId, startedAt.plusSeconds(100), "test")

        val summary = store.sessions(SessionListQuery(limit = 1)).items.single()
        assertEquals(record.sessionId.value, summary.sessionId)
        assertEquals(100, summary.sampleCount)
        assertEquals(99_000, summary.totalElapsedMs)
        assertEquals(99_000, summary.ridingElapsedMs)
        assertTrue(abs(requireNotNull(summary.distanceM) - 495) < 0.001)
        assertEquals(5.0, summary.maxSpeedMps)
        assertEquals(0.99, summary.maxDutyCycle)

        val overview = store.querySeries(
            record.sessionId.value,
            0,
            99_000,
            listOf("packVoltageV", "dutyCycle", "speedKph", "future.energy"),
            12,
        )
        assertTrue(overview.isNotEmpty())
        assertTrue(overview.size <= 12)
        assertTrue(overview.all { it.values.size == 4 })
        assertEquals(
            listOf("future.energy"),
            store.sessionExtensionFields(record.sessionId.value),
        )

        val diagnostic = store.querySampleWindow(
            record.sessionId.value,
            50_000,
            2_000,
            2_000,
            listOf("packVoltageV", "pitchDeg"),
        )
        assertEquals(listOf(48_000L, 49_000L, 50_000L, 51_000L, 52_000L), diagnostic.map { it.elapsedMs })
        assertEquals(
            listOf(0.0, 1.0, 2.0, 3.0, 0.0),
            diagnostic.map { it.values["pitchDeg"] },
        )
        assertEquals(
            100,
            store.sessionEvents(record.sessionId.value, 0, 99_000)
                .count { it.kind == "packageState" },
        )

        val catalogDatabase = SQLiteDatabase.openDatabase(
            File(root, "logs/catalog.sqlite").path,
            null,
            SQLiteDatabase.OPEN_READONLY,
        )
        val sessionFile = catalogDatabase.rawQuery(
            "SELECT file_name FROM session_catalog WHERE session_id = ?",
            arrayOf(record.sessionId.value),
        ).use { cursor ->
            assertTrue(cursor.moveToFirst())
            cursor.getString(0)
        }
        catalogDatabase.close()
        val planDatabase = SQLiteDatabase.openDatabase(
            File(root, "logs/$sessionFile").path,
            null,
            SQLiteDatabase.OPEN_READONLY,
        )
        fun plan(sql: String): String = planDatabase.rawQuery("EXPLAIN QUERY PLAN $sql", null).use { cursor ->
            buildString {
                while (cursor.moveToNext()) append(cursor.getString(3)).append('|')
            }
        }
        assertTrue(plan("SELECT sequence, monotonic_elapsed_ms FROM sample WHERE monotonic_elapsed_ms BETWEEN 0 AND 99000 ORDER BY monotonic_elapsed_ms, sequence LIMIT 12").contains("sample_elapsed"))
        assertTrue(plan("SELECT sequence FROM session_event WHERE (monotonic_elapsed_ms > 1000 OR (monotonic_elapsed_ms = 1000 AND sequence > 1)) ORDER BY monotonic_elapsed_ms, sequence LIMIT 500").contains("session_event_elapsed"))
        assertTrue(plan("SELECT sequence FROM sample WHERE monotonic_elapsed_ms <= 50000 ORDER BY monotonic_elapsed_ms DESC, sequence DESC LIMIT 1").contains("sample_elapsed"))
        assertTrue(plan("SELECT sequence FROM sample WHERE sequence IN (1, 2, 3) ORDER BY sequence LIMIT 3").contains("INTEGER PRIMARY KEY"))
        planDatabase.close()

        val short = store.create(DeviceId("query-fixture"), startedAt.plusSeconds(200))
        store.append(
            short.jobId,
            listOf(
                TelemetrySample(
                    sequence = 1,
                    observedAt = startedAt.plusSeconds(200),
                    elapsedMs = 0,
                    rideState = RideState.IDLE,
                    packVoltageV = 58.0,
                    speedKph = 36.0,
                ),
                TelemetrySample(
                    sequence = 2,
                    observedAt = startedAt.plusSeconds(220),
                    elapsedMs = 20_000,
                    rideState = RideState.IDLE,
                    packVoltageV = 58.0,
                    speedKph = 36.0,
                ),
            ),
        )
        store.close(short.jobId, startedAt.plusSeconds(220), "test")
        val firstPage = store.sessions(
            SessionListQuery(limit = 1, sortField = "totalElapsedMs", sortDescending = false),
        )
        assertEquals(listOf(record.sessionId.value), firstPage.items.map { it.sessionId })
        assertEquals(null, firstPage.nextCursor)
    }

    @Test
    fun cancellingOneRequestDoesNotAbortASiblingReadForTheSameSession() {
        val store = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        val startedAt = Instant.ofEpochMilli(1_700_000_000_000)
        val record = store.create(DeviceId("concurrent-read-fixture"), startedAt)
        store.append(
            record.jobId,
            (0 until 100).map { index ->
                TelemetrySample(
                    sequence = (index + 1).toLong(),
                    observedAt = startedAt.plusMillis(index * 1_000L),
                    elapsedMs = index * 1_000L,
                    rideState = RideState.RIDING,
                    packVoltageV = 60.0 - index / 100.0,
                    packageState = PackageState.entries[index % PackageState.entries.size],
                    speedKph = 18.0,
                )
            },
        )
        store.close(record.jobId, startedAt.plusSeconds(100), "test")

        val firstCancellation = SessionReadCancellation()
        val secondCancellation = SessionReadCancellation()
        val firstRegistered = CountDownLatch(1)
        val secondRegistered = CountDownLatch(1)
        val releaseFirst = CountDownLatch(1)
        val releaseSecond = CountDownLatch(1)
        store.onHistoricalReadRegisteredForTesting = { cancellation ->
            when (cancellation) {
                firstCancellation -> {
                    firstRegistered.countDown()
                    assertTrue(releaseFirst.await(5, TimeUnit.SECONDS))
                }
                secondCancellation -> {
                    secondRegistered.countDown()
                    assertTrue(releaseSecond.await(5, TimeUnit.SECONDS))
                }
            }
        }

        val executor = Executors.newFixedThreadPool(2)
        try {
            val first = executor.submit<List<fail.acheron.floatview.backend.SessionSeriesPoint>> {
                store.withSessionReadCancellation(firstCancellation) {
                    store.querySeries(
                        record.sessionId.value,
                        0,
                        99_000,
                        listOf("packVoltageV"),
                        100,
                    )
                }
            }
            assertTrue(firstRegistered.await(5, TimeUnit.SECONDS))

            val second = executor.submit<List<fail.acheron.floatview.backend.SessionEventRecord>> {
                store.withSessionReadCancellation(secondCancellation) {
                    store.sessionEvents(record.sessionId.value, 0, 99_000)
                }
            }
            assertTrue(secondRegistered.await(5, TimeUnit.SECONDS))

            secondCancellation.cancel()
            releaseSecond.countDown()
            val cancellationFailure = assertThrows(ExecutionException::class.java) { second.get(5, TimeUnit.SECONDS) }
            assertTrue(cancellationFailure.cause is OperationCanceledException)

            releaseFirst.countDown()
            assertEquals(100, first.get(5, TimeUnit.SECONDS).size)
        } finally {
            releaseFirst.countDown()
            releaseSecond.countDown()
            executor.shutdownNow()
            store.onHistoricalReadRegisteredForTesting = null
        }
    }

    @Test
    fun locationCommitBoundariesSurviveSparseWritesRestartAndDeletion() {
        val store = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        val startedAt = Instant.ofEpochMilli(1_700_500_000_000)
        val record = store.create(DeviceId("location-boundary"), startedAt)
        store.appendLocations(record.jobId, listOf(
            LocationObservation(startedAt.plusSeconds(20), 20_000, -33.82, 151.22, 8.0),
            LocationObservation(startedAt, 0, -33.80, 151.20, null),
            LocationObservation(startedAt.plusSeconds(20), 20_000, -33.83, 151.23, 4.0),
            LocationObservation(startedAt.plusSeconds(20), 20_000, -33.82, 151.22, 8.0),
        ))
        val snapshot = store.sessionSnapshot(record.sessionId.value)
        assertEquals(3, snapshot.committedLocationSequence)
        assertEquals(3, snapshot.recordedLocationCount)
        val first = store.queryLocations(
            record.sessionId.value, 0, 20_000, 2, null, snapshot.committedLocationSequence,
        )
        assertEquals(listOf(1L, 2L), first.points.map { it.locationSequence })
        assertTrue(first.nextCursor != null)
        val second = store.queryLocations(
            record.sessionId.value, 0, 20_000, 2, first.nextCursor, snapshot.committedLocationSequence,
        )
        assertEquals(listOf(3L), second.points.map { it.locationSequence })
        assertEquals(listOf("noFix"), first.gaps.map { it.reason })

        store.close(record.jobId, startedAt.plusSeconds(20), "location-boundary")
        store.pruneSession(record.sessionId.value, 5)
        assertEquals(3, store.sessionSnapshot(record.sessionId.value).recordedLocationCount)
        val restarted = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        val persisted = restarted.sessionSnapshot(record.sessionId.value)
        assertEquals(snapshot.committedLocationSequence, persisted.committedLocationSequence)
        assertEquals(3, persisted.recordedLocationCount)
        restarted.deleteSession(record.sessionId.value)
        assertTrue(restarted.sessions(SessionListQuery()).items.none { it.sessionId == record.sessionId.value })
    }

    @Test
    fun deterministicLargeSessionBuilderUsesLosslessSourcePath() {
        val store = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        val startedAt = Instant.ofEpochMilli(1_700_300_000_000)
        val record = store.create(DeviceId("large-fixture"), startedAt)
        var sequence = 1L
        repeat(30) {
            val batch = (0 until 5_000).map { offset ->
                val index = sequence + offset
                TelemetrySample(
                    sequence = index,
                    observedAt = startedAt.plusMillis(index * 1_000L),
                    elapsedMs = index * 1_000L,
                    rideState = if (index < 149_000) RideState.RIDING else RideState.IDLE,
                    packVoltageV = 58.0 - (index % 20) * .01,
                    speedKph = (index % 80).toDouble(),
                    extension = mapOf("fixtureValue" to (index % 17).toDouble()),
                )
            }
            store.append(record.jobId, batch)
            sequence += 5_000
        }
        store.close(record.jobId, startedAt.plusSeconds(200), "large-fixture")
        val source = store.querySeriesSource(
            record.sessionId.value,
            1_000,
            150_000_000,
            listOf("packVoltageV", "fixtureValue"),
            200_000,
            150_000,
        )
        assertEquals(150_000, source.size)
        assertEquals(1L, source.first().sequence)
        assertEquals(150_000L, source.last().sequence)
    }

    @Test
    fun prunePreviewAndCommitRetainProtectedRowsAndAdvanceRevision() {
        val store = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        val startedAt = Instant.ofEpochMilli(1_700_400_000_000)
        val record = store.create(DeviceId("prune-fixture"), startedAt)
        store.append(record.jobId, (0 until 24).map { index ->
            TelemetrySample(
                sequence = (index + 1).toLong(),
                observedAt = startedAt.plusSeconds(index.toLong()),
                elapsedMs = index * 500L,
                rideState = RideState.RIDING,
                packVoltageV = 60.0 - index * .2,
                speedKph = 360.0,
                extension = if (index >= 12) mapOf("late" to index.toDouble()) else emptyMap(),
            )
        })
        store.close(record.jobId, startedAt.plusSeconds(20), "prune")
        val preview = store.prunePreview(record.sessionId.value, 5)
        assertEquals(24, preview.recordedSampleCount)
        assertTrue(preview.estimatedStoredSampleCount <= 24)
        store.pruneSession(record.sessionId.value, 5)
        val snapshot = store.sessionSnapshot(record.sessionId.value)
        assertEquals(1, snapshot.historyRevision)
        assertEquals(24, snapshot.recordedSampleCount)
        assertEquals(preview.estimatedStoredSampleCount, snapshot.storedSampleCount)
        assertTrue(
            store.querySeriesSource(
                record.sessionId.value, 0, 20_000, listOf("speedKph"), 200_000,
                snapshot.committedSampleSequence,
            ).any { it.sequence == 1L },
        )
        assertThrows(SessionStoreException.InvalidRange::class.java) {
            store.prunePreview(record.sessionId.value, 5)
        }
        store.saveMaintenanceJob(
            SessionMaintenanceJobRecord(
                operationId = "prune-test",
                kind = "prune",
                scope = "one",
                state = "running",
                sessionId = record.sessionId.value,
                targetResolutionSeconds = 10,
                upperSessionId = record.sessionId.value,
                lastSessionId = null,
                completedSessions = 0,
                skippedSessions = 0,
                retainedSamples = 0,
                reclaimableBytes = 0,
                error = null,
            ),
        )
        assertEquals("cancel_requested", store.cancelMaintenanceJob("prune-test")?.state)
        assertEquals("prune-test", store.maintenanceJob()?.operationId)
    }

    @Test
    fun outdatedStorageIsDeletedAndRecreated() {
        val logs = File(root, "logs").apply { mkdirs() }
        val stale = File(logs, "old.sqlite").apply { writeText("old data") }
        File(logs, ".schema-version").writeText("outdated")

        val store = SQLiteLogStore(
            context,
            availableBytes = { 2_000_000_000 },
            rootDirectory = logs,
        )
        assertFalse(stale.exists())
        assertTrue(store.sessions(SessionListQuery()).items.isEmpty())
        assertEquals(LogSchema.STORAGE_VERSION, File(logs, ".schema-version").readText())
    }

    @Test
    fun settingsExportsAndIdempotentDeletionUseCanonicalStorage() {
        val store = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        assertEquals(ThemePreference.SYSTEM, store.settings().themePreference)
        assertEquals(100.0, store.settings().minimumSessionDistance, 0.0)
        val settings = store.updateSettings(
            0,
            """{"minimumSessionDistance":80,"idlePersistIntervalMs":2345,"ridingPersistIntervalMs":125,"themePreference":"dark"}""",
        )
        assertEquals(1, settings.revision)
        assertEquals(80.0, settings.minimumSessionDistance, 0.0)
        assertEquals(ThemePreference.DARK, settings.themePreference)
        assertThrows(ProductStoreException.InvalidSettingsPatch::class.java) {
            store.updateSettings(1, """{"themePreference":"twilight"}""")
        }
        assertThrows(ProductStoreException.InvalidSettingsPatch::class.java) {
            store.updateSettings(1, """{"rawPacketCaptureEnabled":true}""")
        }
        assertThrows(ProductStoreException.SettingsRevisionConflict::class.java) {
            store.updateSettings(0, """{"themePreference":"light"}""")
        }

        val base = Instant.ofEpochMilli(1_700_000_000_000)
        val sessionIds = mutableListOf<String>()
        listOf(0L, 120L).forEach { offset ->
            val record = store.create(DeviceId("management-fixture"), base.plusSeconds(offset))
            sessionIds += record.sessionId.value
            store.append(
                record.jobId,
                listOf(
                    TelemetrySample(
                        sequence = 1,
                        observedAt = base.plusSeconds(offset),
                        elapsedMs = 0,
                        rideState = RideState.IDLE,
                        packageState = PackageState.READY,
                        setpointAdjustment = SetpointAdjustment.NONE,
                        packVoltageV = 60.0,
                        dutyCycle = 0.1,
                        speedKph = 0.0,
                        fieldWeakeningCurrentA = 1.25,
                        extension = mapOf("future.energy" to 7.0),
                    ),
                    TelemetrySample(
                        sequence = 2,
                        observedAt = base.plusSeconds(offset + 60),
                        elapsedMs = 60_000,
                        rideState = RideState.RIDING,
                        packageState = PackageState.RUNNING,
                        setpointAdjustment = SetpointAdjustment.DUTY,
                        packVoltageV = 58.0,
                        dutyCycle = 0.5,
                        speedKph = 18.0,
                        fieldWeakeningCurrentA = 2.5,
                        extension = mapOf("future.energy" to 6.0),
                    ),
                ),
            )
            if (offset == 0L) {
                store.appendLocations(record.jobId, listOf(
                    LocationObservation(base, 0, -33.8, 151.2, null),
                    LocationObservation(base.plusSeconds(60), 60_000, -33.81, 151.21, 5.0),
                ))
            }
            store.close(record.jobId, base.plusSeconds(offset + 60), "test")
        }

        store.updateSettings(1, """{"includeGpsLocationWhenExporting":true}""")
        val csv = store.exportSession(sessionIds[0], "csv")
        val csvText = File(csv.filePath).readText()
        assertTrue(csvText.contains("pack_voltage_v"))
        assertTrue(csvText.contains("setpoint_adjustment"))
        assertTrue(csvText.contains("fieldWeakeningCurrentA"))
        assertTrue(csvText.contains("future.energy"))
        assertTrue(csvText.contains("gpsAccuracy,gpsLongitude,gpsLatitude"))
        assertTrue(csvText.contains("151.21"))
        assertFalse(csvText.contains("management-fixture"))
        val archive = store.exportSession(sessionIds[0], "float-view-archive")
        SQLiteDatabase.openDatabase(
            archive.filePath,
            null,
            SQLiteDatabase.OPEN_READONLY,
        ).use { db ->
            assertEquals(
                2L,
                android.database.DatabaseUtils.longForQuery(db, "SELECT COUNT(*) FROM sample", null),
            )
            assertEquals(
                "1",
                db.rawQuery(
                    "SELECT value FROM float_view_archive_metadata WHERE key = 'format_version'",
                    null,
                ).use { cursor ->
                    assertTrue(cursor.moveToFirst())
                    cursor.getString(0)
                },
            )
            val manifest = db.rawQuery(
                "SELECT value FROM float_view_archive_metadata WHERE key = 'session'",
                null,
            ).use { cursor ->
                assertTrue(cursor.moveToFirst())
                org.json.JSONObject(cursor.getString(0))
            }
            assertEquals(2_345, manifest.getLong("requestedIdleMs"))
            assertEquals(125, manifest.getLong("effectiveRidingMs"))
            assertTrue(manifest.get("deviceSnapshot") is org.json.JSONObject)
            assertTrue(manifest.get("boardProfileSnapshot") is org.json.JSONObject)
        }

        store.deleteSession(sessionIds[0])
        store.deleteSession(sessionIds[0])
        try {
            store.session(sessionIds[0])
            throw AssertionError("Deleted session remained readable")
        } catch (_: SessionStoreException.SessionNotFound) {
            // Expected.
        }

        val active = store.create(DeviceId("management-fixture"), base.plusSeconds(300))
        try {
            store.deleteSession(active.sessionId.value)
            throw AssertionError("Active session was deleted")
        } catch (_: SessionStoreException.SessionActive) {
            // Expected.
        }
    }

    @Test
    fun startupResumesAnInterruptedSessionDeletion() {
        val store = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        val record = store.create(DeviceId("delete-recovery"), Instant.EPOCH)
        store.append(
            record.jobId,
            listOf(
                TelemetrySample(
                    sequence = 1,
                    observedAt = Instant.EPOCH,
                    elapsedMs = 0,
                    rideState = RideState.RIDING,
                    speedKph = 18.0,
                ),
                TelemetrySample(
                    sequence = 2,
                    observedAt = Instant.EPOCH.plusSeconds(20),
                    elapsedMs = 20_000,
                    rideState = RideState.RIDING,
                    speedKph = 18.0,
                ),
            ),
        )
        store.close(record.jobId, Instant.EPOCH, "test")
        val logs = File(root, "logs")
        val catalog = SQLiteDatabase.openDatabase(
            File(logs, "catalog.sqlite").path,
            null,
            SQLiteDatabase.OPEN_READWRITE,
        )
        val fileName = catalog.rawQuery(
            "SELECT file_name FROM session_catalog WHERE session_id = ?",
            arrayOf(record.sessionId.value),
        ).use { cursor ->
            assertTrue(cursor.moveToFirst())
            cursor.getString(0)
        }
        catalog.execSQL(
            "UPDATE session_catalog SET pending_delete = 1 WHERE session_id = ?",
            arrayOf(record.sessionId.value),
        )
        catalog.close()
        SQLiteDatabase.deleteDatabase(File(logs, fileName))

        val restarted = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        assertTrue(restarted.sessions(SessionListQuery()).items.isEmpty())
    }

}
