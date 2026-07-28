package fail.acheron.floatview.bluetooth

import android.content.pm.PackageManager
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import fail.acheron.floatview.core.BluetoothEvent
import fail.acheron.floatview.core.BluetoothPermissionState
import fail.acheron.floatview.core.BluetoothRadioState
import fail.acheron.floatview.core.ConnectionState
import fail.acheron.floatview.core.DeviceId
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class T1VescTestDeviceTest {
    @Test
    fun productionAdapterRoundTripsExactBytes() {
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        val terminal = CountDownLatch(1)
        val failure = AtomicReference<String>()
        val selectedDevice = AtomicReference<DeviceId>()
        val notification = AtomicReference<ByteArray>()
        lateinit var adapter: AndroidBluetoothAdapter

        adapter = AndroidBluetoothAdapter(
            context = context,
            permissionRequester = BluetoothPermissionRequester { requested ->
                failure.compareAndSet(null, "unexpected permission request: ${requested.joinToString()}")
                terminal.countDown()
            },
            eventHandler = { event ->
                when (event) {
                    is BluetoothEvent.DeviceFound -> {
                        if (
                            event.advertisedName == EXPECTED_NAME &&
                            selectedDevice.compareAndSet(null, event.deviceId)
                        ) {
                            adapter.connect(event.deviceId)
                        }
                    }
                    is BluetoothEvent.ConnectionChanged -> {
                        if (event.deviceId == selectedDevice.get()) {
                            when (event.state) {
                                ConnectionState.READY ->
                                    adapter.write(event.deviceId, WRITE_BYTES)
                                ConnectionState.DISCONNECTED -> {
                                    if (notification.get() != null) terminal.countDown()
                                }
                                ConnectionState.FAILED -> {
                                    failure.compareAndSet(null, "connection failed")
                                    terminal.countDown()
                                }
                                else -> Unit
                            }
                        }
                    }
                    is BluetoothEvent.Notification -> {
                        if (event.deviceId == selectedDevice.get()) {
                            notification.set(event.bytes)
                            adapter.disconnect(event.deviceId)
                        }
                    }
                    is BluetoothEvent.ScanFailed -> {
                        failure.compareAndSet(null, "scan failed: ${event.reason}")
                        terminal.countDown()
                    }
                    else -> Unit
                }
            },
        )

        val state = adapter.currentState()
        assertEquals(BluetoothRadioState.POWERED_ON, state.radio)
        assertEquals(BluetoothPermissionState.GRANTED, state.scanPermission)
        assertTrue(
            state.connectPermission == BluetoothPermissionState.GRANTED ||
                state.connectPermission == BluetoothPermissionState.NOT_REQUIRED,
        )
        assertEquals(
            PackageManager.PERMISSION_GRANTED,
            context.checkSelfPermission(requiredScanPermission()),
        )

        adapter.startDiscovery()
        assertTrue("BLE round trip timed out", terminal.await(25, TimeUnit.SECONDS))
        selectedDevice.get()?.let(adapter::disconnect)

        assertNull(failure.get(), failure.get())
        assertArrayEquals(NOTIFICATION_BYTES, notification.get())
    }

    private fun requiredScanPermission(): String =
        if (android.os.Build.VERSION.SDK_INT >= 31) {
            android.Manifest.permission.BLUETOOTH_SCAN
        } else {
            android.Manifest.permission.ACCESS_FINE_LOCATION
        }

    companion object {
        private const val EXPECTED_NAME = "FloatView Test VESC"
        private val WRITE_BYTES = byteArrayOf(0x10, 0x20, 0x30, 0x40, 0x50, 0x60)
        private val NOTIFICATION_BYTES =
            byteArrayOf(0xde.toByte(), 0xad.toByte(), 0xbe.toByte(), 0xef.toByte(), 0x01, 0x02)
    }
}
