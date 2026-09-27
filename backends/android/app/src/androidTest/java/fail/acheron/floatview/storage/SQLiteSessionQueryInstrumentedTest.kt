package fail.acheron.floatview.storage

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import android.os.OperationCanceledException
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import fail.acheron.floatview.application.DeviceId
import fail.acheron.floatview.application.LocationObservation
import fail.acheron.floatview.application.PackageState
import fail.acheron.floatview.application.RideState
import fail.acheron.floatview.application.ProductStoreException
import fail.acheron.floatview.application.SessionListQuery
import fail.acheron.floatview.application.SessionMaintenanceJobRecord
import fail.acheron.floatview.application.SessionStoreException
import fail.acheron.floatview.application.SetpointAdjustment
import fail.acheron.floatview.application.TelemetrySample
import fail.acheron.floatview.application.ThemePreference
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
