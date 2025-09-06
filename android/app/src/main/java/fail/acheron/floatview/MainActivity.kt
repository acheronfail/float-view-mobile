package fail.acheron.floatview

import BackendCommand
import android.bluetooth.BluetoothGattCharacteristic
import android.os.Bundle
import android.util.Log
import android.webkit.WebView
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat

fun print(vararg messages: Any) {
    messages.forEach { message -> Log.d("FloatView", message.toString()) }
}

fun printError(vararg messages: Any) {
    messages.forEach { message -> Log.e("FloatView", message.toString()) }
}

class MainActivity : AppCompatActivity() {

    private lateinit var webView: WebView
    private lateinit var localWebView: LocalWebView
    private lateinit var vescBle: VescBle
    private lateinit var vescCmds: VescCommandGenerator

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

        vescBle = VescBle(this)
        vescCmds = VescCommandGenerator()
        vescCmds.setCanId(1)

        // Start the singleton local server
        LocalServer.start(this, webView)
        LocalServer.setHandler { msg ->
            when (msg) {
                is BackendCommand.Scan -> vescBle.startScan()
                is BackendCommand.Connect -> vescBle.connect(msg.address)
                is BackendCommand.Disconnect -> vescBle.disconnect()
                is BackendCommand.Command -> {
                    when (msg.cmd) {
                        is Cmd.CommFwVersion -> vescBle.sendCommand(vescCmds.firmware())
                        is Cmd.CommGetValues -> vescBle.sendCommand(vescCmds.getValues())
                        is Cmd.CommCustomAppData -> {
                            val cmd = msg.cmd.appCommand
                            when (cmd) {
                                is AppCommand.CommandInfo ->
                                        vescBle.sendCommand(
                                                vescCmds.refloatInfo(cmd.version.toInt()),
                                                BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE
                                        )
                                is AppCommand.CommandGetRtdata ->
                                        vescBle.sendCommand(
                                                vescCmds.refloatRtdata(),
                                                BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE
                                        )
                            }
                        }
                    }
                }
                is BackendCommand.LogStart -> {
                    /* TODO */
                }
                is BackendCommand.LogStop -> {
                    /* TODO */
                }
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        LocalServer.stop()
        if (this::localWebView.isInitialized) localWebView.destroy()
        if (this::vescBle.isInitialized) vescBle.disconnect()
    }
}
