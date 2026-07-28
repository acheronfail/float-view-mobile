package fail.acheron.floatview.logging

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import fail.acheron.floatview.core.DeviceId
import fail.acheron.floatview.core.RideState
import fail.acheron.floatview.core.SessionListQuery
import fail.acheron.floatview.core.TelemetrySample
import java.io.File
import java.time.Instant
import kotlin.math.abs
import org.junit.After
import org.junit.Assert.assertEquals
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
                    extension = mapOf("speedKph" to 18.0),
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
            listOf("packVoltageV", "dutyCycle", "speedKph"),
            12,
        )
        assertTrue(overview.isNotEmpty())
        assertTrue(overview.size <= 12)
        assertTrue(overview.all { it.values.size == 3 })

        val diagnostic = store.diagnosticWindow(
            record.sessionId.value,
            50_000,
            2_000,
            2_000,
            listOf("packVoltageV"),
        )
        assertEquals(listOf(48_000L, 49_000L, 50_000L, 51_000L, 52_000L), diagnostic.map { it.elapsedMs })
        assertTrue(store.sessionEvents(record.sessionId.value, 0, 99_000).isEmpty())

        val short = store.create(DeviceId("query-fixture"), startedAt.plusSeconds(200))
        store.append(
            short.jobId,
            listOf(
                TelemetrySample(
                    1,
                    startedAt.plusSeconds(200),
                    500,
                    RideState.IDLE,
                    58.0,
                    null,
                    null,
                    null,
                    null,
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
    fun schemaV2BackfillsExistingSessionSummaries() {
        val logs = File(root, "logs").apply { mkdirs() }
        val catalog = SQLiteDatabase.openOrCreateDatabase(File(logs, "catalog.sqlite"), null)
        executeAsset(catalog, "catalog-v1.sql")
        catalog.execSQL(
            """
            INSERT INTO session_catalog(
              session_id, job_id, device_history_id, device_snapshot_json,
              profile_snapshot_json, profile_revision, file_name, started_at_ms,
              state, integrity, requested_idle_ms, effective_idle_ms,
              requested_riding_ms, effective_riding_ms
            ) VALUES (
              'legacy-session', 'legacy-job', 'legacy-history', '{"displayName":"Legacy board"}',
              '{}', 0, 'session-legacy.sqlite', 0, 'completed', 'complete',
              1000, 1000, 100, 100
            )
            """.trimIndent(),
        )
        catalog.version = 1
        catalog.close()
        val session = SQLiteDatabase.openOrCreateDatabase(File(logs, "session-legacy.sqlite"), null)
        executeAsset(session, "session-v1.sql")
        session.execSQL(
            """
            INSERT INTO sample(
              sequence, observed_at_ms, monotonic_elapsed_ms, ride_state,
              raw_pack_voltage_v, duty_cycle, extension_json
            ) VALUES
              (1, 0, 0, 'idle', 60, 0.1, '{"speedKph":0}'),
              (2, 1000, 1000, 'riding', 59, 0.7, '{"speedKph":18}')
            """.trimIndent(),
        )
        session.close()

        val store = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        val summary = store.session("legacy-session")
        assertEquals("Legacy board", summary.displayName)
        assertEquals(2, summary.sampleCount)
        assertEquals(1_000, summary.totalElapsedMs)
        assertEquals(1_000, summary.ridingElapsedMs)
        assertEquals(0.7, summary.maxDutyCycle)
    }

    private fun executeAsset(db: SQLiteDatabase, name: String) {
        context.assets.open(name).bufferedReader().use { it.readText() }
            .lineSequence()
            .filterNot { it.trimStart().startsWith("--") }
            .filterNot { it.trimStart().startsWith("PRAGMA", ignoreCase = true) }
            .joinToString("\n")
            .split(';')
            .map(String::trim)
            .filter(String::isNotEmpty)
            .forEach(db::execSQL)
    }
}
