package fail.acheron.floatview.storage

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import fail.acheron.floatview.backend.DeviceId
import fail.acheron.floatview.backend.PackageState
import fail.acheron.floatview.backend.RideState
import fail.acheron.floatview.backend.ProductStoreException
import fail.acheron.floatview.backend.SessionListQuery
import fail.acheron.floatview.backend.SessionStoreException
import fail.acheron.floatview.backend.SetpointAdjustment
import fail.acheron.floatview.backend.TelemetrySample
import fail.acheron.floatview.backend.ThemePreference
import java.io.File
import java.time.Instant
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

        val diagnostic = store.diagnosticWindow(
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

        val short = store.create(DeviceId("query-fixture"), startedAt.plusSeconds(200))
        store.append(
            short.jobId,
	            listOf(
	                TelemetrySample(
	                    sequence = 1,
	                    observedAt = startedAt.plusSeconds(200),
	                    elapsedMs = 500,
	                    rideState = RideState.IDLE,
	                    packVoltageV = 58.0,
	                ),
	            ),
        )
        store.close(short.jobId, startedAt.plusSeconds(201), "test")
        val firstPage = store.sessions(
            SessionListQuery(limit = 1, sortField = "totalElapsedMs", sortDescending = false),
        )
        assertEquals(listOf(short.sessionId.value), firstPage.items.map { it.sessionId })
        val secondPage = store.sessions(
            SessionListQuery(
                cursor = requireNotNull(firstPage.nextCursor),
                limit = 1,
                minimumTotalElapsedMs = 1_000,
                sortField = "totalElapsedMs",
                sortDescending = false,
            ),
        )
        assertEquals(listOf(record.sessionId.value), secondPage.items.map { it.sessionId })
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
    fun settingsLinkingExportsAndIdempotentDeletionUseCanonicalStorage() {
        val store = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        assertEquals(ThemePreference.SYSTEM, store.settings().themePreference)
        val settings = store.updateSettings(
            0,
            """{"automaticSessionLinkingEnabled":true,"sessionLinkWindowMs":7200000,"idlePersistIntervalMs":2345,"ridingPersistIntervalMs":125,"themePreference":"dark"}""",
        )
        assertEquals(1, settings.revision)
        assertTrue(settings.automaticSessionLinkingEnabled)
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
            store.close(record.jobId, base.plusSeconds(offset + 60), "test")
        }

        val group = store.sessionGroups().single()
        assertEquals(sessionIds, group.sessionIds)
        assertEquals(120_000, group.totalElapsedMs)
        assertEquals(group, store.sessionGroup(group.sessionGroupId))

        val csv = store.exportSession(sessionIds[0], "csv")
        val csvText = File(csv.filePath).readText()
        assertTrue(csvText.contains("pack_voltage_v"))
        assertTrue(csvText.contains("setpoint_adjustment"))
        assertTrue(csvText.contains("fieldWeakeningCurrentA"))
        assertTrue(csvText.contains("future.energy"))
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
        assertTrue(store.sessionGroups().isEmpty())
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
