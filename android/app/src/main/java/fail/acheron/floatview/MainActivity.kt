package fail.acheron.floatview

import android.annotation.SuppressLint
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.webkit.WebView
import androidx.appcompat.app.AppCompatActivity
import kotlinx.serialization.json.*

class MainActivity : AppCompatActivity() {

    private lateinit var webView: WebView
    private lateinit var localWebView: LocalWebView
    private val TAG = "FloatView_MainActivity"

    private lateinit var vescBle: VescBle
    private val mainHandler = Handler(Looper.getMainLooper())

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        Log.d(TAG, "MainActivity onCreate started")

        localWebView = LocalWebView(this)
        webView = localWebView.init()
        setContentView(webView)

        vescBle = VescBle(this)

        // Start the singleton local server
        LocalServer.start(this, webView)

        var bleDevices: List<BluetoothDevice> = listOf()
        LocalServer.addRoute("/api/ble/scan") { ctx: Context ->
            if (!vescBle.requestPermissionsIfNeeded(this, 1001)) {
                ctx.setJson(errorResponse("permissions not granted"))
                ctx.send()
                return@addRoute
            }

            vescBle.startScan(
                    listOf(VescBle.VescService),
                    3_000L,
                    object : VescBle.ScanListener {
                        @SuppressLint("MissingPermission")
                        override fun onScanStopped(devices: List<BluetoothDevice>) {
                            Log.d(TAG, "BLE scan complete. Devices count: ${devices.size}")
                            // Execute the response on a background thread to avoid NetworkOnMainThreadException
                            bleDevices = devices
                            Thread {
                                        ctx.setJson(successResponse(buildJsonArray {
                                            bleDevices.forEach { device ->
                                                add(buildJsonObject {
                                                    put("name", device.name ?: "Unknown")
                                                    put("id", device.address)
                                                })
                                            }
                                        }))
                                        ctx.send()
                                    }
                                    .start()
                        }

                        override fun onError(message: String) {
                            Log.e(TAG, "BLE scan error: $message")
                            ctx.setJson(errorResponse(message)).send()
                        }
                    }
            )
        }

        LocalServer.addRoute("/api/ble/connect") { ctx: Context ->
            val bleConnectRequest = Json.decodeFromString<BleConnectRequest>(ctx.getRequestBody())
            val connected = connectToDevice(bleConnectRequest.deviceId)
            if (connected) {
                ctx.setJson(successResponse(buildJsonObject { put("status", "connected") }))
            } else {
                ctx.setJson(errorResponse("not found"))
            }
            ctx.send()
        }

    }

    private fun connectToDevice(address: String): Boolean {
        Log.d(TAG, "Connecting to device: $address")

        return vescBle.connect(
                address,
                listOf(VescBle.VescService),
                object : VescBle.ConnectionListener {
                    override fun onConnected(gatt: BluetoothGatt) {
                        Log.d(TAG, "Connected to device: ${gatt.device.address}")
                    }

                    override fun onReady(gatt: BluetoothGatt) {
                        Log.d(TAG, "Device ready: ${gatt.device.address}")

                        // Log all available services and characteristics
                        logAvailableCharacteristics(gatt)

                        // Wait 5 seconds then disconnect
                        mainHandler.postDelayed(
                                {
                                    Log.d(TAG, "Disconnecting from device after 5 seconds")
                                    vescBle.disconnect()
                                },
                                5000
                        )
                    }

                    override fun onDisconnected() {
                        Log.d(TAG, "Disconnected from device: ${address}")
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
        if (this::localWebView.isInitialized) localWebView.destroy()
    }
}
