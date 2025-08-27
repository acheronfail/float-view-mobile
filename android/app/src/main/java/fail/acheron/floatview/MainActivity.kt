package fail.acheron.floatview

import android.bluetooth.BluetoothDevice
import android.os.Bundle
import android.util.Log
import android.webkit.WebView
import androidx.appcompat.app.AppCompatActivity
import java.util.UUID

class MainActivity : AppCompatActivity() {

    private lateinit var webView: WebView
    private lateinit var webViewModule: WebViewModule
    private val TAG = "FloatView_MainActivity"

    private lateinit var bluetoothModule: BluetoothModule

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        Log.d(TAG, "MainActivity onCreate started")

        // Initialize WebView via module
        webViewModule = WebViewModule(this)
        webView = webViewModule.init()
        setContentView(webView) // Keep layout simple for now

        // Start the singleton local server
        LocalServer.start(this, webView)

        // Initialize Bluetooth module (permissions may prompt user)
        bluetoothModule = BluetoothModule(this)
        // If permissions already granted, start scan immediately; otherwise request then scan in
        // callback
        if (bluetoothModule.requestPermissionsIfNeeded(this, 1001)) {
            startBleScan()
        }
    }

    private val targetServiceUuid: UUID = UUID.fromString("6e400001-b5a3-f393-e0a9-e50e24dcca9e")

    private fun startBleScan() {
        bluetoothModule.startScan(
                listOf(targetServiceUuid),
                10_000L,
                object : BluetoothModule.ScanListener {
                    override fun onDeviceFound(device: BluetoothDevice) {
                        Log.d(
                                TAG,
                                "BLE device with target service found: ${'$'}{device.address} (${ '$'}{device.name})"
                        )
                    }

                    override fun onScanStopped(devices: List<BluetoothDevice>) {
                        Log.d(TAG, "BLE scan complete. Devices count: ${'$'}{devices.size}")
                    }

                    override fun onError(message: String) {
                        Log.e(TAG, "BLE scan error: ${'$'}message")
                    }
                }
        )
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
                startBleScan()
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
