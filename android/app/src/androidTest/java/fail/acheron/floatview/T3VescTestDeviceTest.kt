package fail.acheron.floatview

import android.content.Context
import android.content.Intent
import android.database.sqlite.SQLiteDatabase
import android.os.SystemClock
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import fail.acheron.floatview.bluetooth.AndroidBluetoothAdapter
import fail.acheron.floatview.bluetooth.BluetoothPermissionRequester
import fail.acheron.floatview.core.ApplicationService
import fail.acheron.floatview.core.BluetoothEvent
import fail.acheron.floatview.core.ClockPort
import fail.acheron.floatview.core.ConnectionState
import fail.acheron.floatview.core.DeviceId
import fail.acheron.floatview.core.RideState
import fail.acheron.floatview.core.SamplingSettings
import fail.acheron.floatview.core.SessionListQuery
import fail.acheron.floatview.core.SystemScheduler
import fail.acheron.floatview.core.TelemetrySample
import fail.acheron.floatview.logging.SQLiteLogStore
import fail.acheron.floatview.protocol.VescProtocolService
import java.io.File
import java.time.Instant
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class T3VescTestDeviceTest {
    @Test
    fun productionStackCreatesDurableSession() {
        val mode = Mode.parse(
            InstrumentationRegistry.getArguments().getString("mode") ?: "ride",
        )
        val context = ApplicationProvider.getApplicationContext<Context>()
        val activity = InstrumentationRegistry.getInstrumentation().startActivitySync(
            Intent(context, MainActivity::class.java)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
        )
        InstrumentationRegistry.getInstrumentation().waitForIdleSync()
        val terminal = CountDownLatch(1)
        val failure = AtomicReference<String>()
        val selectedDevice = AtomicReference<DeviceId>()
        val telemetryCount = AtomicInteger()
        val ridingTelemetryCount = AtomicInteger()
        val idleTelemetryCount = AtomicInteger()
        val sawFootpadWarning = AtomicBoolean()
        val sawUnexpectedDisconnectReason = AtomicBoolean()
        val disconnected = AtomicBoolean()
        val relay = EventRelay()
        val store = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        lateinit var adapter: AndroidBluetoothAdapter
        lateinit var service: ApplicationService

        fun completeIfReady() {
            if (
                disconnected.get() &&
                (mode == Mode.RIDE || sawUnexpectedDisconnectReason.get())
            ) {
                terminal.countDown()
            }
        }

        adapter = AndroidBluetoothAdapter(
            context = context,
            permissionRequester = BluetoothPermissionRequester { requested ->
                failure.compareAndSet(
                    null,
                    "unexpected permission request: ${requested.joinToString()}",
                )
                terminal.countDown()
            },
            eventHandler = relay::emit,
        )
        service = ApplicationService(
            bluetooth = adapter,
            clock = AndroidTestClock,
            logStore = store,
            productStore = store,
            sessionStore = store,
            protocol = VescProtocolService(),
            scheduler = SystemScheduler(),
            samplingSettings = SamplingSettings(
                reconnectDurationMs = if (mode == Mode.UNEXPECTED_DISCONNECT) 0 else 300_000,
            ),
        ) { event ->
            when (event) {
                is TelemetrySample -> {
                    val count = telemetryCount.incrementAndGet()
                    when (event.rideState) {
                        RideState.RIDING -> ridingTelemetryCount.incrementAndGet()
                        RideState.IDLE -> idleTelemetryCount.incrementAndGet()
                        RideState.UNKNOWN -> Unit
                    }
                    if (event.footpadState == 1) sawFootpadWarning.set(true)
                    if (mode == Mode.RIDE && count == mode.expectedTelemetryCount) {
                        service.disconnect()
                    }
                }
                is ConnectionState -> {
                    if (event == ConnectionState.DISCONNECTED) {
                        disconnected.set(true)
                        completeIfReady()
                    }
                }
                is String -> {
                    if (event == "unexpected_disconnect") {
                        sawUnexpectedDisconnectReason.set(true)
                        completeIfReady()
                    }
                }
                is Throwable -> {
                    failure.compareAndSet(null, event.message ?: event.javaClass.simpleName)
                    terminal.countDown()
                }
            }
        }
        relay.handler = service::onBluetoothEvent

        val state = adapter.currentState()
        assertEquals("Bluetooth radio must be on", "POWERED_ON", state.radio.name)
        assertTrue(
            "Bluetooth scan permission must be granted",
            state.scanPermission.name == "GRANTED" ||
                state.scanPermission.name == "NOT_REQUIRED",
        )

        try {
            service.startDiscovery()
            val discoveryDeadline = SystemClock.elapsedRealtime() + DISCOVERY_TIMEOUT_MS
            while (selectedDevice.get() == null && SystemClock.elapsedRealtime() < discoveryDeadline) {
                val found = service.knownDevices.firstOrNull {
                    it.advertisedName == EXPECTED_NAME
                }
                if (found != null && selectedDevice.compareAndSet(null, found.id)) {
                    service.connect(found.id)
                } else {
                    Thread.yield()
                }
            }
            assertNotNull("test VESC was not discovered", selectedDevice.get())
            assertTrue(
                "T3 ${mode.wireName} scenario timed out",
                terminal.await(mode.timeoutSeconds, TimeUnit.SECONDS),
            )
            assertEquals(null, failure.get())
            assertEquals(mode.expectedTelemetryCount, telemetryCount.get())
            assertFalse("connection remained active", service.activeConnectionSnapshot() != null)
            assertEquals("session remained active", null, store.active())

            val summary = store.sessions(SessionListQuery(limit = 2)).items.single()
            val closeReason = closeReason(context, summary.sessionId)
            assertEquals("completed", summary.state)
            assertEquals("complete", summary.integrity)
            assertNotNull(summary.endedAt)
            assertTrue("no riding duration persisted", summary.ridingElapsedMs > 0)
            when (mode) {
                Mode.RIDE -> {
                    assertTrue("insufficient ride samples: ${summary.sampleCount}", summary.sampleCount >= 150)
                    assertTrue("ride duration too short: ${summary.totalElapsedMs}", summary.totalElapsedMs >= 18_000)
                    assertEquals("explicit_disconnect", closeReason)
                    assertTrue("footpad warning was not decoded", sawFootpadWarning.get())
                    val window = store.diagnosticWindow(
                        summary.sessionId,
                        summary.totalElapsedMs / 2,
                        summary.totalElapsedMs,
                        summary.totalElapsedMs,
                        listOf("footpadState"),
                    )
                    assertTrue(
                        "footpad warning was not durable",
                        window.any { it.values["footpadState"] == 1.0 },
                    )
                }
                Mode.UNEXPECTED_DISCONNECT -> {
                    assertTrue("insufficient disconnect samples: ${summary.sampleCount}", summary.sampleCount >= 3)
                    assertTrue("disconnect duration too short: ${summary.totalElapsedMs}", summary.totalElapsedMs >= 1_000)
                    assertEquals("unexpected_disconnect", closeReason)
                    assertTrue(
                        "unexpected disconnect reason was not published",
                        sawUnexpectedDisconnectReason.get(),
                    )
                }
            }
            assertEquals("ok", sqliteIntegrity(context))

            val evidence = JSONObject()
                .put("sessionId", summary.sessionId)
                .put("sessionState", summary.state)
                .put("sessionIntegrity", summary.integrity)
                .put("sampleCount", summary.sampleCount)
                .put("ridingElapsedMs", summary.ridingElapsedMs)
                .put("totalElapsedMs", summary.totalElapsedMs)
                .put("telemetryCount", telemetryCount.get())
                .put("ridingTelemetryCount", ridingTelemetryCount.get())
                .put("idleTelemetryCount", idleTelemetryCount.get())
                .put("sawFootpadWarning", sawFootpadWarning.get())
                .put("sawUnexpectedDisconnectReason", sawUnexpectedDisconnectReason.get())
                .put("closeReason", closeReason)
                .put("mode", mode.wireName)
                .put("sqliteIntegrity", "ok")
                .put("activeConnection", false)
                .put("activeSession", false)
            File(context.cacheDir, RESULT_FILE).writeText("${evidence}\n")
        } finally {
            service.activeConnectionSnapshot()?.let { service.disconnect() }
            adapter.stopDiscovery()
            activity.runOnUiThread(activity::finish)
        }
    }

    private fun closeReason(context: Context, sessionId: String): String {
        val catalog = File(context.filesDir, "float-view/logs/catalog.sqlite")
        return SQLiteDatabase.openDatabase(
            catalog.path,
            null,
            SQLiteDatabase.OPEN_READONLY,
        ).use { database ->
            database.rawQuery(
                "SELECT close_reason FROM session_catalog WHERE session_id = ?",
                arrayOf(sessionId),
            ).use { cursor ->
                check(cursor.moveToFirst()) { "session catalog row missing" }
                cursor.getString(0)
            }
        }
    }

    private fun sqliteIntegrity(context: Context): String {
        val files = File(context.filesDir, "float-view/logs")
            .walkTopDown()
            .filter { it.isFile && it.extension == "sqlite" }
            .toList()
        check(files.isNotEmpty()) { "no SQLite evidence found" }
        return if (
            files.all { file ->
                SQLiteDatabase.openDatabase(
                    file.path,
                    null,
                    SQLiteDatabase.OPEN_READONLY,
                ).use { database ->
                    database.rawQuery("PRAGMA integrity_check", null).use { cursor ->
                        cursor.moveToFirst() && cursor.getString(0) == "ok"
                    }
                }
            }
        ) {
            "ok"
        } else {
            "failed"
        }
    }

    private enum class Mode(
        val wireName: String,
        val expectedTelemetryCount: Int,
        val timeoutSeconds: Long,
    ) {
        RIDE("ride", 168, 40),
        UNEXPECTED_DISCONNECT("unexpected-disconnect", 5, 25);

        companion object {
            fun parse(value: String): Mode =
                entries.firstOrNull { it.wireName == value }
                    ?: error("unsupported mode: $value")
        }
    }

    private class EventRelay {
        @Volatile
        var handler: ((BluetoothEvent) -> Unit)? = null

        fun emit(event: BluetoothEvent) {
            handler?.invoke(event)
        }
    }

    private object AndroidTestClock : ClockPort {
        override fun monotonicMillis(): Long = SystemClock.elapsedRealtime()
        override fun now(): Instant = Instant.now()
    }

    companion object {
        private const val EXPECTED_NAME = "FloatView Test VESC"
        private const val RESULT_FILE = "t3-android-result.json"
        private const val DISCOVERY_TIMEOUT_MS = 15_000L
    }
}
