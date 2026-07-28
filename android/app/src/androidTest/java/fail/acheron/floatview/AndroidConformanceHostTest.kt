package fail.acheron.floatview

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import fail.acheron.floatview.core.ApplicationService
import fail.acheron.floatview.core.BluetoothPermissionState
import fail.acheron.floatview.core.BluetoothPort
import fail.acheron.floatview.core.BluetoothRadioState
import fail.acheron.floatview.core.BluetoothState
import fail.acheron.floatview.core.ClockPort
import fail.acheron.floatview.core.DeviceId
import fail.acheron.floatview.core.EventSink
import fail.acheron.floatview.core.RideState
import fail.acheron.floatview.core.TelemetrySample
import fail.acheron.floatview.logging.SQLiteLogStore
import java.io.File
import java.time.Instant
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.json.JSONObject
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AndroidConformanceHostTest {
    @Test
    fun servesSeededApiUntilTheHostRequestsShutdown() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val testRoot = File(context.cacheDir, "r5-conformance-logs").also {
            it.deleteRecursively()
        }
        val store = SQLiteLogStore(
            context,
            availableBytes = { Long.MAX_VALUE },
            rootDirectory = testRoot,
        )
        val seed = store.create(DeviceId("conformance-profile-fixture"), Instant.EPOCH)
        store.append(
            seed.jobId,
            (0 until 60).map { index ->
                TelemetrySample(
                    sequence = (index + 1).toLong(),
                    observedAt = Instant.ofEpochSecond(index.toLong()),
                    elapsedMs = index * 1_000L,
                    rideState = if (index < 5) RideState.IDLE else RideState.RIDING,
                    packVoltageV = 60.0 - index.toDouble() / 20.0,
                    dutyCycle = index.toDouble() / 100.0,
                    controllerTempC = 30.0 + index.toDouble() / 10.0,
                    motorTempC = 28.0 + index.toDouble() / 10.0,
                    footpadState = if (index < 5) 0 else 3,
                    extension = mapOf(
                        "motorCurrentA" to index.toDouble(),
                        "batteryCurrentA" to index.toDouble() / 2.0,
                        "erpm" to (index * 100).toDouble(),
                        "speedKph" to index.toDouble() / 3.0,
                    ),
                )
            },
        )
        store.close(seed.jobId, Instant.ofEpochSecond(60), "test_seed")
        val service = ApplicationService(
            bluetooth = ConformanceBluetooth,
            clock = ConformanceClock,
            logStore = store,
            productStore = store,
            sessionStore = store,
            events = EventSink {},
        )
        val server = LoopbackServer(context.assets, service)
        val stopped = CountDownLatch(1)
        val receiver = object : BroadcastReceiver() {
            override fun onReceive(context: Context?, intent: Intent?) {
                if (intent?.action == STOP_ACTION) stopped.countDown()
            }
        }
        if (Build.VERSION.SDK_INT >= 33) {
            context.registerReceiver(receiver, IntentFilter(STOP_ACTION), Context.RECEIVER_NOT_EXPORTED)
        } else {
            @Suppress("DEPRECATION")
            context.registerReceiver(receiver, IntentFilter(STOP_ACTION))
        }
        val readiness = File(context.cacheDir, READINESS_FILE)
        try {
            readiness.delete()
            server.start()
            readiness.writeText(
                JSONObject()
                    .put("port", server.port)
                    .put("origin", server.origin)
                    .put("savedDeviceId", store.savedDevices().single().savedDeviceId)
                    .toString() + "\n",
            )
            assertTrue(
                "Node conformance host did not request shutdown",
                stopped.await(120, TimeUnit.SECONDS),
            )
        } finally {
            server.stop()
            context.unregisterReceiver(receiver)
            readiness.delete()
            testRoot.deleteRecursively()
        }
    }

    private object ConformanceBluetooth : BluetoothPort {
        private val state = BluetoothState(
            BluetoothRadioState.POWERED_ON,
            BluetoothPermissionState.GRANTED,
            BluetoothPermissionState.GRANTED,
        )

        override fun currentState() = state
        override fun requestPermission() = Unit
        override fun startDiscovery() = Unit
        override fun stopDiscovery() = Unit
        override fun connect(deviceId: DeviceId) = Unit
        override fun write(deviceId: DeviceId, bytes: ByteArray) = Unit
        override fun disconnect(deviceId: DeviceId) = Unit
    }

    private object ConformanceClock : ClockPort {
        override fun monotonicMillis() = 0L
        override fun now() = Instant.EPOCH
    }

    companion object {
        const val READINESS_FILE = "r5-android-conformance.json"
        const val STOP_ACTION = "fail.acheron.floatview.R5_CONFORMANCE_STOP"
    }
}
