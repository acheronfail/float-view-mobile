package fail.acheron.floatview

import android.os.Bundle
import android.util.Log
import android.webkit.WebView
import androidx.appcompat.app.AppCompatActivity
import kotlinx.serialization.json.*

class MainActivity : AppCompatActivity() {

    private lateinit var webView: WebView
    private lateinit var localWebView: LocalWebView
    private val TAG = "FloatView_MainActivity"

    private lateinit var vescManager: VescManager

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        Log.d(TAG, "MainActivity onCreate started")

        localWebView = LocalWebView(this)
        webView = localWebView.init()
        setContentView(webView)

        vescManager = VescManager(this)

        // Start the singleton local server
        LocalServer.start(this, webView)

        setupWebSocketHandlers()

        vescManager.setListener(
                object : VescManager.VescManagerListener {
                    override fun onScanDeviceFound(name: String?, address: String) {
                        LocalServer.webSocket.broadcastMessage(
                                wsMessage(
                                        "scan",
                                        "device",
                                        buildJsonObject {
                                            put("name", name)
                                            put("address", address)
                                        }
                                )
                        )
                    }

                    override fun onScanComplete() {
                        LocalServer.webSocket.broadcastMessage(
                                wsMessage(
                                        "scan",
                                        "status",
                                        buildJsonObject { put("status", "complete") }
                                )
                        )
                    }

                    override fun onScanError(message: String) {
                        LocalServer.webSocket.broadcastMessage(
                                wsMessage(
                                        "scan",
                                        "error",
                                        buildJsonObject { put("message", message) }
                                )
                        )
                    }

                    override fun onConnectionStatusChanged(
                            status: VescManager.ConnectionStatus,
                            address: String?
                    ) {
                        when (status) {
                            VescManager.ConnectionStatus.CONNECTING -> {
                                LocalServer.webSocket.broadcastMessage(
                                        wsMessage(
                                                "connect",
                                                "status",
                                                buildJsonObject { put("status", "connecting") }
                                        )
                                )
                            }
                            VescManager.ConnectionStatus.CONNECTED -> {
                                LocalServer.webSocket.broadcastMessage(
                                        wsMessage(
                                                "connect",
                                                "status",
                                                buildJsonObject { put("status", "connected") }
                                        )
                                )
                            }
                            VescManager.ConnectionStatus.READY -> {
                                LocalServer.webSocket.broadcastMessage(
                                        wsMessage(
                                                "connect",
                                                "status",
                                                buildJsonObject { put("status", "ready") }
                                        )
                                )
                            }
                            VescManager.ConnectionStatus.DISCONNECTED -> {
                                Log.e(TAG, "Disconnected from device")
                            }
                        }
                    }

                    override fun onConnectionError(message: String) {
                        LocalServer.webSocket.broadcastMessage(
                                wsMessage(
                                        "connect",
                                        "error",
                                        buildJsonObject { put("message", message) }
                                )
                        )
                    }

                    override fun onVescResponse(command: Int, data: ByteArray) {
                        when (command) {
                            COMM_FW_VERSION -> {

                                val parser = VescBuffer(data)
                                val major = parser.readUInt8()
                                val minor = parser.readUInt8()
                                var name = parser.readString()
                                if (name.isEmpty()) name = "Unknown"

                                var uuid: String? = null
                                if (parser.size() - parser.index >= 12) {
                                    val sb = StringBuilder()
                                    for (i in 0 until 12) {
                                        val v = parser.readUInt8()
                                        if (v < 10) {
                                            sb.append("0")
                                            sb.append(v.toString(16))
                                        } else {
                                            sb.append(v.toString(16).uppercase())
                                        }
                                    }
                                    uuid = sb.toString()
                                }
                                var isPaired: Boolean? = null
                                if (parser.size() - parser.index >= 1) {
                                    isPaired = parser.readUInt8() != 0
                                }
                                var betaVersion: Int? = null
                                if (parser.size() - parser.index >= 1) {
                                    betaVersion = parser.readUInt8()
                                }
                                var hwType: Int? = null
                                if (parser.size() - parser.index >= 1) {
                                    hwType = parser.readUInt8()
                                }

                                LocalServer.webSocket.broadcastMessage(
                                        wsMessage(
                                                "data",
                                                "firmware",
                                                buildJsonObject {
                                                    put("major", major)
                                                    put("minor", minor)
                                                    put("name", name)
                                                    put("uuid", uuid)
                                                    put("isPaired", isPaired)
                                                    put("betaVersion", betaVersion)
                                                    put("hwType", hwType)
                                                }
                                        )
                                )
                            }
                            else -> {
                                Log.w(
                                        TAG,
                                        "Unhandled VESC response - Command: $command, Data: ${data.joinToString(" ") { "%02X".format(it) }}"
                                )
                            }
                        }
                    }

                    override fun onCommandError(command: Int, message: String) {
                        Log.e(TAG, "Command error - Command: $command, Error: $message")
                        vescManager.disconnect()
                        LocalServer.webSocket.broadcastMessage(
                                wsMessage(
                                        "connect",
                                        "disconnect",
                                        buildJsonObject { put("reason", "command error") }
                                )
                        )
                    }
                }
        )
    }

    private fun setupWebSocketHandlers() {
        setupScanHandler()
        setupConnectHandler()
        setupDataHandler()
    }

    private fun setupScanHandler() {
        LocalServer.addHandler("scan") { _ ->
            val topic = "scan"

            if (!vescManager.startScan()) {
                LocalServer.webSocket.broadcastMessage(
                        wsMessage(
                                topic,
                                "error",
                                buildJsonObject { put("message", "Failed to start scan") }
                        )
                )
            }

            LocalServer.webSocket.broadcastMessage(
                    wsMessage(topic, "status", buildJsonObject { put("status", "start") })
            )
        }
    }

    private fun setupConnectHandler() {
        LocalServer.addHandler("connect") { req ->
            val jsonAddress = req["address"]
            if (jsonAddress == null) {
                LocalServer.webSocket.broadcastMessage(
                        wsMessage(
                                "connect",
                                "error",
                                buildJsonObject { put("message", "no address") }
                        )
                )
                return@addHandler
            }

            val address = jsonAddress.jsonPrimitive.contentOrNull
            if (address == null) {
                LocalServer.webSocket.broadcastMessage(
                        wsMessage(
                                "connect",
                                "error",
                                buildJsonObject { put("message", "no address") }
                        )
                )
                return@addHandler
            }

            vescManager.connectAndGetFirmware(address)
        }

        LocalServer.addHandler("disconnect") { _ ->
            vescManager.disconnect()
            LocalServer.webSocket.broadcastMessage(
                    wsMessage(
                            "connect",
                            "disconnect",
                            buildJsonObject { put("reason", "websocket requested") }
                    )
            )
        }
    }

    private fun setupDataHandler() {
        LocalServer.addHandler("data") { req ->
            val shouldLog = req["log"]!!.jsonPrimitive.boolean
            if (!shouldLog) {
                // TODO: stop logging interval
                return@addHandler
            }

            @Suppress("UNUSED_VARIABLE") val intervalMs = req["intervalMs"]!!.jsonPrimitive.double
            // TODO: start sending VESC log data back with interval: ${intervalMs}ms
            LocalServer.webSocket.broadcastMessage(
                    wsMessage(
                            "data",
                            "log",
                            buildJsonObject {
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
                                put("state", "riding")
                                put("stateRaw", 0)
                                put("tempMosfet", 0)
                                put("tempMotor", 0)
                                put("time", 0)
                                put("truePitch", 0)
                                put("voltage", 0)
                                put("wh", 0)
                            }
                    )
            )
        }
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
                Log.d(TAG, "All BLE permissions granted")
            } else {
                Log.w(TAG, "BLE permissions denied")
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        LocalServer.stop()
        if (this::localWebView.isInitialized) localWebView.destroy()
        if (this::vescManager.isInitialized) vescManager.disconnect()
    }
}
