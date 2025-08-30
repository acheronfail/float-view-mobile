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

        LocalServer.addHandler("scan") { _, reply ->
            val topic = "scan"

            if (!vescBle.requestPermissionsIfNeeded(this, 1001)) {
                reply(
                        wsMessage(
                                topic,
                                "error",
                                buildJsonObject { put("message", "no permissions") }
                        )
                )
                return@addHandler
            }

            vescBle.startScan(
                    listOf(VescBle.VescService),
                    3_000L,
                    object : VescBle.ScanListener {
                        @SuppressLint("MissingPermission")
                        override fun onDeviceFound(device: BluetoothDevice) {
                            Thread {
                                        reply(
                                                wsMessage(
                                                        topic,
                                                        "device",
                                                        buildJsonObject {
                                                            put("name", device.name)
                                                            put("address", device.address)
                                                        }
                                                )
                                        )
                                    }
                                    .start()
                        }

                        @SuppressLint("MissingPermission")
                        override fun onScanStopped(devices: List<BluetoothDevice>) {
                            Log.d(TAG, "BLE scan complete. Devices count: ${devices.size}")
                            Thread { reply(wsMessage(topic, "complete", buildJsonObject {})) }
                                    .start()
                        }

                        override fun onError(message: String) {
                            Log.e(TAG, "BLE scan error: $message")
                            Thread {
                                        reply(
                                                wsMessage(
                                                        topic,
                                                        "error",
                                                        buildJsonObject { put("message", message) }
                                                )
                                        )
                                    }
                                    .start()
                        }
                    }
            )
        }

        LocalServer.addHandler("connect") { req, reply ->
            val topic = "connect"

            val jsonAddress = req["address"]
            if (jsonAddress == null) {
                reply(wsMessage(topic, "error", buildJsonObject { put("message", "no address") }))
                return@addHandler
            }

            val address = jsonAddress.jsonPrimitive.contentOrNull
            if (address == null) {
                reply(wsMessage(topic, "error", buildJsonObject { put("message", "no address") }))
                return@addHandler
            }

            reply(wsMessage(topic, "status", buildJsonObject { put("status", "start") }))
            val didStartConnecting =
                    vescBle.connect(
                            address,
                            listOf(VescBle.VescService),
                            object : VescBle.ConnectionListener {
                                override fun onConnected(gatt: BluetoothGatt) {
                                    Log.d(TAG, "Connected to device: ${gatt.device.address}")
                                    reply(
                                            wsMessage(
                                                    topic,
                                                    "status",
                                                    buildJsonObject { put("status", "connected") }
                                            )
                                    )
                                }

                                override fun onReady(gatt: BluetoothGatt) {
                                    Log.d(TAG, "Device ready: ${gatt.device.address}")
                                    reply(
                                            wsMessage(
                                                    topic,
                                                    "ready",
                                                    buildJsonObject { put("address", address) }
                                            )
                                    )

                                    // TODO: investigate - `requestConnectionPriorityForDevice -
                                    // HIGH`
                                    // TODO: detect disconnects (out of range, etc)
                                    // TODO: reconnection logic/timers
                                    // TODO: start monitoring logs:
                                    // TODO: monitorCharacteristicForDevice for constant updates of
                                    // the char (VescCharacteristicsTx)
                                    // Log all available services and characteristics
                                    logAvailableCharacteristics(gatt)

                                    // Wait 5 seconds then disconnect
                                    mainHandler.postDelayed(
                                            {
                                                Log.d(
                                                        TAG,
                                                        "Disconnecting from device after 5 seconds"
                                                )
                                                vescBle.disconnect()
                                            },
                                            5000
                                    )
                                }

                                override fun onDisconnected() {
                                    reply(
                                            wsMessage(
                                                    topic,
                                                    "disconnect",
                                                    buildJsonObject { put("address", address) }
                                            )
                                    )
                                }

                                override fun onError(message: String) {
                                    reply(
                                            wsMessage(
                                                    topic,
                                                    "error",
                                                    buildJsonObject { put("message", message) }
                                            )
                                    )
                                }
                            }
                    )
            if (didStartConnecting) {
                reply(wsMessage(topic, "status", buildJsonObject { put("status", "connecting") }))
            } else {
                reply(
                        wsMessage(
                                topic,
                                "error",
                                buildJsonObject { put("message", "no device found") }
                        )
                )
            }
        }

        LocalServer.addHandler("data") { req, reply ->
            val shouldLog = req["log"]!!.jsonPrimitive.boolean
            if (!shouldLog) {
                // TODO: stop logging interval
                return@addHandler
            }

            val intervalMs = req["intervalMs"]!!.jsonPrimitive.double
            // TODO: start sending VESC log data back
            reply(wsMessage("data", "log", buildJsonObject {
                put("adc1", 0)
                put("adc2", 0)
                put("ah", 0)
                put("altitude", 0)
                put("currentBattery", 0)
                put("currentMotor", 0)
                put("distance", 0)
                put("duty", 0)
                put("gpsAccuracy", 0)
                put("gpsLatitude", 0)
                put("gpsLongitude", 0)
                put("motorFault", 0)
                put("pitch", 0)
                put("roll", 0)
                put("speed", 0)
                put("state", 0)
                put("stateRaw", 0)
                put("tempMosfet", 0)
                put("tempMotor", 0)
                put("time", 0)
                put("truePitch", 0)
                put("voltage", 0)
                put("wh", 0)
            }))
        }
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
