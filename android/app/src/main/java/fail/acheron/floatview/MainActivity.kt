package fail.acheron.floatview

import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.webkit.WebView
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import kotlinx.serialization.json.*

fun print(vararg messages: Any) {
    messages.forEach { message -> Log.d("FloatView", message.toString()) }
}

fun printError(vararg messages: Any) {
    messages.forEach { message -> Log.e("FloatView", message.toString()) }
}

class MainActivity : AppCompatActivity() {

    private lateinit var webView: WebView
    private lateinit var localWebView: LocalWebView
    private lateinit var vescManager: VescManager

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Enable edge-to-edge display
        WindowCompat.setDecorFitsSystemWindows(window, false)

        localWebView = LocalWebView(this)
        webView = localWebView.init()

        // Create a container to handle insets
        val container = FrameLayout(this)
        container.setBackgroundColor(
                getColor(android.R.color.black)
        ) // Set background color for safe areas
        container.addView(
                webView,
                FrameLayout.LayoutParams(
                        FrameLayout.LayoutParams.MATCH_PARENT,
                        FrameLayout.LayoutParams.MATCH_PARENT
                )
        )
        setContentView(container)

        // Apply window insets as padding to the container
        ViewCompat.setOnApplyWindowInsetsListener(container) { view, insets ->
            val systemBars = insets.getInsets(WindowInsetsCompat.Type.systemBars())
            val displayCutout = insets.getInsets(WindowInsetsCompat.Type.displayCutout())

            // Take the maximum of system bars and display cutout for each side
            val topInset = maxOf(systemBars.top, displayCutout.top)
            val bottomInset = maxOf(systemBars.bottom, displayCutout.bottom)
            val leftInset = maxOf(systemBars.left, displayCutout.left)
            val rightInset = maxOf(systemBars.right, displayCutout.right)
            view.setPadding(leftInset, topInset, rightInset, bottomInset)

            insets
        }

        vescManager = VescManager(this)

        // Start the singleton local server
        LocalServer.start(this, webView)

        LocalServer.addHandler("scan") { _ -> vescManager.startScan() }

        LocalServer.addHandler("connect") { req ->
            val jsonAddress = req["address"]
            if (jsonAddress == null) {
                LocalServer.broadcastMessage(
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
                LocalServer.broadcastMessage(
                        wsMessage(
                                "connect",
                                "error",
                                buildJsonObject { put("message", "no address") }
                        )
                )
                return@addHandler
            }

            vescManager.connect(address)
        }

        LocalServer.addHandler("disconnect") { _ ->
            vescManager.disconnect()
            LocalServer.broadcastMessage(
                    wsMessage(
                            "connect",
                            "disconnect",
                            buildJsonObject { put("reason", "websocket requested") }
                    )
            )
        }

        LocalServer.addHandler("command") { req ->
            val command = req["command"]!!.jsonPrimitive.int
            val payload = req["payload"]?.jsonArray?.map { it.jsonPrimitive.int } ?: listOf()
            Handler(Looper.getMainLooper())
                    .postDelayed({ vescManager.sendCommand(command, payload) }, 1000)
        }

        LocalServer.addHandler("data") { req ->
            val shouldLog = req["log"]!!.jsonPrimitive.boolean
            if (!shouldLog) {
                // TODO: stop logging interval
                return@addHandler
            }

            @Suppress("UNUSED_VARIABLE") val intervalMs = req["intervalMs"]!!.jsonPrimitive.double
            // TODO: start sending VESC log data back with interval: ${intervalMs}ms
            LocalServer.broadcastMessage(
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

    override fun onDestroy() {
        super.onDestroy()
        LocalServer.stop()
        if (this::localWebView.isInitialized) localWebView.destroy()
        if (this::vescManager.isInitialized) vescManager.disconnect()
    }
}
