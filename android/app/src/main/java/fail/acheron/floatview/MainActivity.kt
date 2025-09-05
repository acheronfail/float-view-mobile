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
    private lateinit var vescBluetooth: VescBluetooth
    private lateinit var vescCommandGenerator: VescCommandGenerator

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

        vescBluetooth = VescBluetooth(this)
        vescCommandGenerator = VescCommandGenerator()

        // Start the singleton local server
        LocalServer.start(this, webView)

        LocalServer.addHandler("scan") { _ -> vescBluetooth.startScan() }

        LocalServer.addHandler("connect") { req ->
            val jsonAddress = req["address"]
            if (jsonAddress == null) {
                LocalServer.broadcastMessage(BackendMessage.connectError("no address"))
                return@addHandler
            }

            val address = jsonAddress.jsonPrimitive.contentOrNull
            if (address == null) {
                LocalServer.broadcastMessage(BackendMessage.connectError("no address"))
                return@addHandler
            }

            vescBluetooth.connect(address)
        }

        LocalServer.addHandler("disconnect") { req ->
            val address = req["address"]?.jsonPrimitive?.contentOrNull
            if (address == null) {
                LocalServer.broadcastMessage(BackendMessage.disconnectError(null, "no address"))
                return@addHandler
            }

            vescBluetooth.disconnect()
            // NOTE: backend only supports a single device for now
            LocalServer.broadcastMessage(BackendMessage.disconnectComplete(address))
        }

        LocalServer.addHandler("command") { req ->
            val commandInt = req["command"]?.jsonPrimitive?.intOrNull
            if (commandInt == null) {
                return@addHandler LocalServer.broadcastMessage(BackendMessage.commandError(null, "no command"))
            }
            val command = CommPacketId.fromInt(commandInt)
            if (command == null) {
                return@addHandler LocalServer.broadcastMessage(BackendMessage.commandError(null, "no command"))
            }

            // TODO: parse commands here and map to vescCommandGenerator[method]

            val payload = req["payload"]?.jsonArray?.map { it.jsonPrimitive.int } ?: listOf()
            Handler(Looper.getMainLooper())
                    .postDelayed({ vescBluetooth.sendCommand(command, payload) }, 1000)
        }

        LocalServer.addHandler("data") { req ->
            val address = req["address"]!!.jsonPrimitive.content
            val shouldLog = req["log"]!!.jsonPrimitive.boolean
            if (!shouldLog) {
                // TODO: stop logging interval
                return@addHandler
            }

            @Suppress("UNUSED_VARIABLE") val intervalMs = req["intervalMs"]!!.jsonPrimitive.double
            // TODO: start sending VESC log data back with interval: ${intervalMs}ms
            LocalServer.broadcastMessage(
                    BackendMessage.logData(
                            address,
                            DataObject.build {
                                state = "riding"
                                speed = 42.2
                            }
                    )
            )
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        LocalServer.stop()
        if (this::localWebView.isInitialized) localWebView.destroy()
        if (this::vescBluetooth.isInitialized) vescBluetooth.disconnect()
    }
}
