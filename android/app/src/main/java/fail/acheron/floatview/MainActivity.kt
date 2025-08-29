package fail.acheron.floatview

import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCharacteristic
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.webkit.WebView
import androidx.appcompat.app.AppCompatActivity
import java.util.UUID

class MainActivity : AppCompatActivity() {

    private lateinit var webView: WebView
    private lateinit var webViewModule: WebViewModule
    private val TAG = "FloatView_MainActivity"

    private lateinit var bluetoothModule: BluetoothModule
    private val mainHandler = Handler(Looper.getMainLooper())

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        Log.d(TAG, "MainActivity onCreate started")

        webViewModule = WebViewModule(this)
        webView = webViewModule.init()
        setContentView(webView)

        bluetoothModule = BluetoothModule(this)

        // Start the singleton local server
        LocalServer.start(this, webView)
        LocalServer.addRoute("/api/status") { ctx: LocalServerContext ->
            ctx.setJson("""{"status":"all good"}""")
            ctx.send()
        }
        LocalServer.addRoute("/api/ble/scan") { ctx: LocalServerContext ->
            if (!bluetoothModule.requestPermissionsIfNeeded(this, 1001)) {
                ctx.setJson("""{"error":"invalid permissions","kind":"permissions"}""")
                ctx.send()
                return@addRoute
            }

            val devices = mutableListOf<BluetoothDevice>()
            bluetoothModule.startScan(
                listOf(BluetoothModule.VescService),
                3_000L,
                object : BluetoothModule.ScanListener {
                    override fun onDeviceFound(device: BluetoothDevice) {
                        Log.d(
                                TAG,
                                "BLE device with target service found: ${device.address} (${device.name})"
                        )
                        devices.add(device)
                    }

                    override fun onScanStopped(devices: List<BluetoothDevice>) {
                        Log.d(TAG, "BLE scan complete. Devices count: ${devices.size}")
                        // Execute the response on a background thread to avoid NetworkOnMainThreadException
                        Thread {
                            ctx.setJson("[${devices.joinToString(",") { """{"name":"${it.name}","id":"${it.address}"}""" }}]")
                            ctx.send()
                        }.start()
                    }

                    override fun onError(message: String) {
                        Log.e(TAG, "BLE scan error: $message")
                    }
                }
        )
        }
    }

    private fun connectToDevice(device: BluetoothDevice) {
        Log.d(TAG, "Connecting to device: ${device.address}")

        bluetoothModule.connect(
                device,
                listOf(BluetoothModule.VescService),
                object : BluetoothModule.ConnectionListener {
                    override fun onConnected(gatt: BluetoothGatt) {
                        Log.d(TAG, "Connected to device: ${device.address}")
                    }

                    override fun onReady(gatt: BluetoothGatt) {
                        Log.d(TAG, "Device ready: ${device.address}")

                        // Log all available services and characteristics
                        logAvailableCharacteristics(gatt)

                        // Wait 5 seconds then disconnect
                        mainHandler.postDelayed(
                                {
                                    Log.d(TAG, "Disconnecting from device after 5 seconds")
                                    bluetoothModule.disconnect()
                                },
                                5000
                        )
                    }

                    override fun onDisconnected() {
                        Log.d(TAG, "Disconnected from device: ${device.address}")
                    }

                    override fun onError(message: String) {
                        Log.e(TAG, "Connection error: $message")
                    }
                }
        )
    }

    private fun logAvailableCharacteristics(gatt: BluetoothGatt) {
        Log.d(TAG, "=== Available Services and Characteristics ===")

        val services = gatt.services
        Log.d(TAG, "Total services found: ${services.size}")

        for (service in services) {
            Log.d(TAG, "Service UUID: ${service.uuid}")
            Log.d(TAG, "  Service type: ${service.type}")

            val characteristics = service.characteristics
            Log.d(TAG, "  Characteristics count: ${characteristics.size}")

            for (characteristic in characteristics) {
                Log.d(TAG, "    Characteristic UUID: ${characteristic.uuid}")
                Log.d(TAG, "    Properties: ${getPropertiesString(characteristic.properties)}")
                Log.d(TAG, "    Permissions: ${characteristic.permissions}")

                val descriptors = characteristic.descriptors
                if (descriptors.isNotEmpty()) {
                    Log.d(TAG, "    Descriptors count: ${descriptors.size}")
                    for (descriptor in descriptors) {
                        Log.d(TAG, "      Descriptor UUID: ${descriptor.uuid}")
                    }
                }
            }
        }
        Log.d(TAG, "=== End of Services and Characteristics ===")
    }

    private fun getPropertiesString(properties: Int): String {
        val props = mutableListOf<String>()
        if (properties and android.bluetooth.BluetoothGattCharacteristic.PROPERTY_READ != 0)
                props.add("READ")
        if (properties and android.bluetooth.BluetoothGattCharacteristic.PROPERTY_WRITE != 0)
                props.add("WRITE")
        if (properties and
                        android.bluetooth.BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE !=
                        0
        )
                props.add("WRITE_NO_RESPONSE")
        if (properties and android.bluetooth.BluetoothGattCharacteristic.PROPERTY_NOTIFY != 0)
                props.add("NOTIFY")
        if (properties and android.bluetooth.BluetoothGattCharacteristic.PROPERTY_INDICATE != 0)
                props.add("INDICATE")
        if (properties and android.bluetooth.BluetoothGattCharacteristic.PROPERTY_BROADCAST != 0)
                props.add("BROADCAST")
        if (properties and android.bluetooth.BluetoothGattCharacteristic.PROPERTY_EXTENDED_PROPS !=
                        0
        )
                props.add("EXTENDED_PROPS")
        if (properties and android.bluetooth.BluetoothGattCharacteristic.PROPERTY_SIGNED_WRITE != 0)
                props.add("SIGNED_WRITE")
        return if (props.isEmpty()) "NONE" else props.joinToString(", ")
    }

    override fun onRequestPermissionsResult(
            requestCode: Int,
            permissions: Array<out String>,
            grantResults: IntArray
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == 1001) {
            val allGranted =
                    grantResults.isNotEmpty() &&
                            grantResults.all {
                                it == android.content.pm.PackageManager.PERMISSION_GRANTED
                            }
            if (allGranted) {
                Log.d(TAG, "All BLE permissions granted; starting scan")
            } else {
                Log.w(TAG, "BLE permissions denied; cannot start scan")
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        LocalServer.stop()
        if (this::webViewModule.isInitialized) webViewModule.destroy()
    }
}
